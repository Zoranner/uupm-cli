import axios from 'axios';
import ora from 'ora';
import * as unzipper from 'unzipper';
import * as fs from 'fs';
import * as path from 'path';
import fse from 'fs-extra';
import semver from 'semver';
import xml2js from 'xml2js';
import { Queue } from 'typescript-collections';

const unityScope = 'com.nuget';
const nugetBaseUrl = 'http://pkg.rd.kim/repository/nuget-group/v3/content/';

let pascalName: string;
let kebabName: string;
let targetVersion: string;
let nugetPkgPath: string;
let targetFramework: string;
let unityPkgName: string;
let unityPkgPath: string;
let dependentQueue: Queue<any> = new Queue<any>();

const loadStepSpinner = async (
  steps: {
    startTitle: string;
    stepAction: () => Promise<string>;
    errorAction: (() => Promise<void>) | null;
    finallyAction: (() => Promise<void>) | null;
  }[]
) => {
  for (const step of steps) {
    const spinner = ora({
      text: step.startTitle,
      color: 'yellow'
    }).start();

    try {
      spinner.succeed(await step.stepAction());
    } catch (error: any) {
      if (step.errorAction) {
        await step.errorAction();
      }
      spinner.fail(`[BREAK] ${error}`);
      break;
    } finally {
      if (step.finallyAction) {
        await step.finallyAction();
      }
    }
  }
};

const getNuGet2Unity = async (name: string) => {
  const nugetPkgInfo = name.split('@');
  pascalName = nugetPkgInfo[0];
  kebabName = pascalName.toLowerCase();
  nugetPkgPath = path.join(process.cwd(), `${pascalName}.nupkg`);
  unityPkgName = `${unityScope}.${kebabName}`;

  await loadStepSpinner([
    // Step 1: looking for package
    {
      startTitle: 'Looking for nuget package...',
      stepAction: async () => {
        if (nugetPkgInfo.length > 1) {
          targetVersion = await lookingForVersion(nugetPkgInfo[1]);
        } else {
          targetVersion = await lookingForLatestVersion();
        }
        unityPkgPath = path.join(
          process.cwd(),
          `${unityPkgName}@${targetVersion}`
        );
        return `The version ${targetVersion} of ${pascalName} found.`;
      },
      errorAction: null,
      finallyAction: null
    },
    // Step 2: download package
    {
      startTitle: 'Downloading nuget package...',
      stepAction: async () => {
        await downloadNuPackage();
        return `Download ${pascalName}@${targetVersion} complete.`;
      },
      errorAction: null,
      finallyAction: null
    },
    // Step 3: convert package info
    {
      startTitle: 'Converting package info...',
      stepAction: async () => {
        await convertPackageInfo();
        return `Converted package info to package.json.`;
      },
      errorAction: async () => {
        await fse.remove(nugetPkgPath);
      },
      finallyAction: null
    },
    // Step 4: extract package
    {
      startTitle: 'Extracting package to local...',
      stepAction: async () => {
        await extractSpecificFiles();
        return `Extracted to ${unityPkgName}@${targetVersion}.`;
      },
      errorAction: null,
      finallyAction: async () => {
        await fse.remove(nugetPkgPath);
      }
    }
  ]);
};

const lookingForVersion = async (version: string) => {
  const response = await axios.get(`${nugetBaseUrl}/${kebabName}/index.json`);
  const versions = response.data.versions;

  const neededVersions = versions.map(
    (version: string) =>
      version.includes('rc') ? version.replace('-rc', '') : version // 替换 'rc'
  );
  //   console.log();
  //   console.log(neededVersions);
  // 检查是否存在该版本
  if (neededVersions.includes(version)) {
    return version;
  } else {
    throw new Error(`The version ${version} of ${pascalName} does not exist.`);
  }
};

const lookingForLatestVersion = async () => {
  const response = await axios.get(`${nugetBaseUrl}/${kebabName}/index.json`);
  const versions = response.data.versions;
  const neededVersions = versions
    .filter((version: string) => !version.includes('preview')) // 过滤掉 'preview'
    .map(
      (version: string) =>
        version.includes('rc') ? version.replace('-rc', '') : version // 替换 'rc'
    );

  // 使用语义化版本库（如 semver）来对版本进行排序
  neededVersions.sort((a: number, b: number) => semver.rcompare(a, b));
  // 返回最新的稳定版本
  return neededVersions[0];
};

const downloadNuPackage = async () => {
  // 构建下载URL
  const nugetPkgUrl = `${nugetBaseUrl}/${kebabName}/${targetVersion}/${kebabName}.${targetVersion}.nupkg`;
  const { data } = await axios({
    method: 'GET',
    url: nugetPkgUrl,
    responseType: 'stream'
  });
  // 创建文件写入流
  const writer = fs.createWriteStream(nugetPkgPath);
  data.pipe(writer);

  await new Promise((resolve, reject) => {
    writer.on('finish', resolve);
    writer.on('error', reject);
  });
};

const convertPackageInfo = async () => {
  const directory = await unzipper.Open.file(nugetPkgPath);
  const nuspecFile = directory.files.find(
    (file: any) => file.path === `${pascalName}.nuspec`
  );
  const nuspecContent = (await nuspecFile.buffer()).toString('utf8');
  const parser = new xml2js.Parser();
  const result = await parser.parseStringPromise(nuspecContent);

  // 提取所需信息
  const metadata = result.package.metadata[0];
  const packageInfo = {
    name: unityPkgName,
    displayName: pascalName,
    version: targetVersion,
    unity: '2021.3',
    author: {
      name: 'NuGet',
      email: 'rd@kimo-tech.com'
    },
    description: metadata.description[0],
    type: 'library',
    keywords: metadata.tags ? metadata.tags[0].split(' ') : [],
    license: metadata.license ? metadata.license[0]._ : 'Unknown',
    dependencies: {},
    repository: metadata.repository
      ? {
          url: metadata.repository[0].$.url,
          type: metadata.repository[0].$.type,
          revision: metadata.repository[0].$.commit
        }
      : {}
  };

  const frameworksGroup = metadata.dependencies[0].group;
  if (!frameworksGroup) {
    throw new Error('The library does not support Unity.');
  }

  const neededGroup = frameworksGroup
    .filter((group: any) => group.$.targetFramework.startsWith('.NETStandard'))
    .sort((a: any, b: any) =>
      b.$.targetFramework.localeCompare(a.$.targetFramework)
    )[0];
  if (!neededGroup) {
    throw new Error('The library does not support Unity.');
  }
  // const dependencies = neededGroup.dependency.map((dep: any) => ({
  //   id: `${unityScope}.${dep.$.id.toLowerCase()}`,
  //   version: dep.$.version
  // }));
  // packageInfo.dependencies = dependencies.reduce((acc: any, dep: any) => {
  //   acc[dep.id] = dep.version;
  //   return acc;
  // }, {});

  let dependencies: any = {}; // 初始化dependencies为空对象
  neededGroup.dependency.forEach((dep: any) => {
    const dependencyId = `${unityScope}.${dep.$.id.toLowerCase()}`;
    dependencies[dependencyId] = dep.$.version;
    dependentQueue.enqueue(`${dep.$.id}@${dep.$.version}`);
  });
  targetFramework = neededGroup.$.targetFramework;
  packageInfo.dependencies = dependencies;
  console.log(dependentQueue);

  // 将信息保存至package.json文件
  const packageJsonPath = path.join(unityPkgPath, 'package.json');
  if (fs.existsSync(unityPkgPath)) {
    await fse.remove(unityPkgPath);
  }
  fs.mkdirSync(unityPkgPath, { recursive: true });
  await fs.promises.writeFile(
    packageJsonPath,
    JSON.stringify(packageInfo, null, 2),
    'utf8'
  );

  return packageJsonPath;
};

const extractSpecificFiles = async () => {
  const libraryPath = `lib/${targetFramework.toLowerCase().slice(1)}/`;

  // 使用 unzipper 解压文件
  return new Promise((resolve, reject) => {
    fs.createReadStream(nugetPkgPath)
      .pipe(unzipper.Parse())
      .on('entry', (entry: any) => {
        const fileName = entry.path;
        const isRootFile =
          fileName.indexOf('/') === -1 && fileName !== '[Content_Types].xml';
        const isLibFile = fileName.startsWith(libraryPath);

        if (isRootFile || isLibFile) {
          const fullPath = path.join(unityPkgPath, fileName);
          // 确保目录存在
          fs.mkdirSync(path.dirname(fullPath), { recursive: true });
          // 解压文件
          entry.pipe(fs.createWriteStream(fullPath));
        } else {
          entry.autodrain(); // 忽略其他文件或目录
        }
      })
      .on('close', resolve) // 解压完成
      .on('error', reject);
  });
};

export { getNuGet2Unity };
