import axios from 'axios';
import * as unzipper from 'unzipper';
import * as fs from 'fs';
import * as path from 'path';
import fse from 'fs-extra';
import xml2js from 'xml2js';
import { Queue } from 'typescript-collections';
import { loadStepSpinner } from './step-spinner.js';
import { readConfigs } from '../configs/config-base.js';

export default class NuGetPackageResolver {
  private nugetBaseUrl = '';
  private recursion: boolean = false;
  private pascalName: string = '';
  private kebabName: string = '';
  private targetVersion: string = '';
  private nugetPkgPath: string = '';
  private targetFramework: string = '';
  private unityPkgName: string = '';
  private unityPkgPath: string = '';
  private dependentQueue: Queue<any> = new Queue<any>();

  private PACKAGES_PATH: string = 'Packages';
  private UNIT_SCOPE = 'org.nuget';
  private OFFICIAL_NUGET_BASE_URL = 'https://api.nuget.org/v3-flatcontainer/';

  constructor() {}

  async getNugetBaseUrl() {
    const globalConfig = readConfigs();
    try {
      const nugetConfig = globalConfig.registry.nuget;
      const nugetSourceUrl = nugetConfig.source[nugetConfig.default];
      const response = await axios.get(nugetSourceUrl);
      const resources = response.data.resources;
      for (let i = 0; i < resources.length; i++) {
        if (resources[i]['@type'] === 'PackageBaseAddress/3.0.0') {
          this.nugetBaseUrl = resources[i]['@id'];
          break;
        }
      }
    } catch (e) {
      this.nugetBaseUrl = this.OFFICIAL_NUGET_BASE_URL;
      console.log(e);
    }
  }

  async recursionResolve(name: string): Promise<void> {
    this.recursion = true;
    this.dependentQueue.enqueue(name);
    while (this.dependentQueue.size() > 0) {
      const currentName = this.dependentQueue.dequeue();
      console.log(`> ${currentName}`);
      await this.singleResolve(currentName);
    }
  }

  async singleResolve(name: string): Promise<void> {
    const nugetPkgInfo = name.split('@');
    this.pascalName = nugetPkgInfo[0];
    this.kebabName = this.pascalName.toLowerCase();
    this.nugetPkgPath = path.join(
      this.PACKAGES_PATH,
      `${this.pascalName}.nupkg`
    );
    this.unityPkgName = `${this.UNIT_SCOPE}.${this.kebabName}`;

    await loadStepSpinner([
      // Step 1: looking for package
      {
        startTitle: 'Looking for nuget package...',
        stepAction: async () => {
          if (nugetPkgInfo.length > 1) {
            this.targetVersion = await this.lookingForVersion(nugetPkgInfo[1]);
          } else {
            this.targetVersion = await this.lookingForLatestVersion();
          }
          this.unityPkgPath = path.join(
            this.PACKAGES_PATH,
            `${this.unityPkgName}-${this.targetVersion}`
          );
          return `The version ${this.targetVersion} of ${this.pascalName} found.`;
        },
        errorAction: null,
        finallyAction: null
      },
      // Step 2: download package
      {
        startTitle: 'Downloading nuget package...',
        stepAction: async () => {
          await this.downloadNuPackage();
          return `Download ${this.pascalName}@${this.targetVersion} complete.`;
        },
        errorAction: null,
        finallyAction: null
      },
      // Step 3: convert package info
      {
        startTitle: 'Converting package info...',
        stepAction: async () => {
          await this.convertPackageInfo();
          return `Converted package info to package.json.`;
        },
        errorAction: async () => {
          await fse.remove(this.nugetPkgPath);
        },
        finallyAction: null
      },
      // Step 4: extract package
      {
        startTitle: 'Extracting package to local...',
        stepAction: async () => {
          await this.extractSpecificFiles();
          return `Extracted to ${this.unityPkgName}@${this.targetVersion}.`;
        },
        errorAction: null,
        finallyAction: async () => {
          await fse.remove(this.nugetPkgPath);
        }
      }
    ]);
  }

  private async lookingForVersion(version: string): Promise<string> {
    const response = await axios.get(
      `${this.nugetBaseUrl}/${this.kebabName}/index.json`
    );
    const versions = response.data.versions;

    // const neededVersions = versions.map(
    //   (version: string) =>
    //     version.includes('rc') ? version.replace('-rc', '') : version // 替换 'rc'
    // );
    // console.log();
    // console.log(neededVersions);
    // 检查是否存在该版本
    if (versions.includes(version)) {
      return version;
    } else {
      throw new Error(
        `The version ${version} of ${this.pascalName} does not exist.`
      );
    }
  }

  private async lookingForLatestVersion(): Promise<string> {
    const response = await axios.get(
      `${this.nugetBaseUrl}/${this.kebabName}/index.json`
    );
    const versions = response.data.versions;
    const neededVersions = versions
      .filter((version: string) => !version.includes('-preview')) // 过滤掉 'preview'
      .filter((version: string) => !version.includes('-beta')) // 过滤掉 'beta'
      .filter((version: string) => !version.includes('-rc')); // 过滤掉 'beta'
    // console.log();
    // console.log(neededVersions);

    // // 使用语义化版本库对版本进行排序
    // neededVersions.sort((a: number, b: number) => semver.rcompare(a, b));
    // 返回最新的稳定版本
    return neededVersions[neededVersions.length - 1];
  }

  private async downloadNuPackage(): Promise<void> {
    // 构建下载URL
    const nugetPkgUrl = `${this.nugetBaseUrl}/${this.kebabName}/${this.targetVersion}/${this.kebabName}.${this.targetVersion}.nupkg`;
    const { data } = await axios({
      method: 'GET',
      url: nugetPkgUrl,
      responseType: 'stream'
    });
    // 创建文件写入流
    const writer = fs.createWriteStream(this.nugetPkgPath);
    data.pipe(writer);

    await new Promise((resolve, reject) => {
      writer.on('finish', resolve);
      writer.on('error', reject);
    });
  }

  private async convertPackageInfo(): Promise<void> {
    const directory = await unzipper.Open.file(this.nugetPkgPath);
    const nuspecFile = directory.files.find(
      (file: any) => file.path === `${this.pascalName}.nuspec`
    );
    const nuspecContent = (await nuspecFile.buffer()).toString('utf8');
    const parser = new xml2js.Parser();
    const result = await parser.parseStringPromise(nuspecContent);

    // 提取所需信息
    const metadata = result.package.metadata[0];
    const packageInfo = {
      name: this.unityPkgName,
      displayName: this.pascalName,
      version: this.targetVersion,
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

    const frameworksGroup = metadata.dependencies
      ? metadata.dependencies[0].group
      : undefined;
    if (!frameworksGroup) {
      throw new Error('The library does not support Unity.');
    }

    const neededGroup = frameworksGroup
      .filter((group: any) =>
        group.$.targetFramework.startsWith('.NETStandard')
      )
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
    this.targetFramework = neededGroup.$.targetFramework;
    // console.log();
    // console.log(this.targetFramework);

    if (neededGroup.dependency) {
      let dependencies: any = {}; // 初始化dependencies为空对象
      neededGroup.dependency.forEach((dep: any) => {
        const dependencyId = `${this.UNIT_SCOPE}.${dep.$.id.toLowerCase()}`;
        dependencies[dependencyId] = dep.$.version;
        if (this.recursion) {
          const packageName = `${dep.$.id}@${dep.$.version}`;
          if (!this.dependentQueue.contains(packageName)) {
            this.dependentQueue.enqueue(packageName);
          }
        }
      });
      packageInfo.dependencies = dependencies;
    }

    // console.log(this.dependentQueue);

    // 将信息保存至package.json文件
    const packageJsonPath = path.join(this.unityPkgPath, 'package.json');
    if (fs.existsSync(this.unityPkgPath)) {
      await fse.remove(this.unityPkgPath);
    }
    fs.mkdirSync(this.unityPkgPath, { recursive: true });
    await fs.promises.writeFile(
      packageJsonPath,
      JSON.stringify(packageInfo, null, 2),
      'utf8'
    );
  }

  private async extractSpecificFiles(): Promise<void> {
    const libraryPath = `lib/${this.targetFramework.toLowerCase().slice(1)}/`;
    const runtimesPath = `runtimes/`;

    // 使用 unzipper 解压文件
    return new Promise((resolve, reject) => {
      fs.createReadStream(this.nugetPkgPath)
        .pipe(unzipper.Parse())
        .on('entry', (entry: any) => {
          const fileName = entry.path;
          const isRootFile =
            fileName.indexOf('/') === -1 && fileName !== '[Content_Types].xml';
          const isLibraryFile = fileName.startsWith(libraryPath);
          const isRuntimesFile = fileName.startsWith(runtimesPath);

          if (isRootFile || isLibraryFile || isRuntimesFile) {
            const fullPath = path.join(this.unityPkgPath, fileName);
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
  }
}
