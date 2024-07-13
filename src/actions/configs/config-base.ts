import * as fs from 'fs';
import * as yaml from 'yaml';
import * as os from 'os';
import { exec, spawnSync } from 'child_process';
import {
  GlobalConfig,
  EditorVersions
} from '../../interfaces/global-config.js';

// 获取用户主目录
const userHomeDir = os.homedir();
const configFile = `${userHomeDir}/.upmrc`;
const editorBaseDirs = [
  'C:/Program Files/',
  'C:/Program Files/Unity/Hub/Editor/'
];

// 初始化 Configs 配置
const initConfigs = () => {
  const initialConfigs: GlobalConfig = {
    registry: {
      origin: {
        default: 'Unity',
        source: {
          Unity: 'https://packages.unity.com'
        }
      },
      nuget: {
        default: 'Nuget',
        source: {
          Nuget: 'https://api.nuget.org/v3/'
        }
      }
    },
    editor: {
      default: 'defaultNuget',
      version: {
        Nuget: 'https://api.nuget.org/v3/'
      }
    }
  };
  return initialConfigs;
};

// 扫描已知的 Editor 版本
const scanEditorVersions = (): EditorVersions => {
  let versions: EditorVersions = {};
  editorBaseDirs.forEach((editorBaseDir) => {
    // console.log(editorBaseDir);
    fs.readdirSync(editorBaseDir)
      .map((childDir) => `${editorBaseDir}${childDir}`)
      .forEach((childDir) => {
        const unityExePath = `${childDir}/Editor/Unity.exe`;
        // console.log(unityExePath);
        if (fs.existsSync(unityExePath)) {
          // console.log(unityExePath);
          // 通过 -version 命令，获取版本信息
          const version = spawnSync(unityExePath, ['-version'], {
            encoding: 'utf8',
            stdio: ['overlapped', 'pipe', 'overlapped']
          });
          console.log(JSON.stringify(version));
          // exec(`"${unityExePath}" -version`, (error, stdout, stderr) => {
          //   if (error) {
          //     console.error(`Error: ${error}`);
          //     return;
          //   }
          //   if (stderr) {
          //     console.error(`Stderr: ${stderr}`);
          //     return;
          //   }
          //   console.log(`Version and Author: ${stdout}`);
          // });
          versions[version.stdout] = unityExePath;
        }
      });
  });
  // const layer2Dirs = fs.readdirSync(editorBaseDir);
  // layer2Dirs
  //   .map((layer2Dir) => `${editorBaseDir}${layer2Dir}/`)
  //   .forEach((layer2Dir) => {
  //     console.log(layer2Dir);
  //     try {
  //       fs.readdirSync(layer2Dir, { recursive: true })
  //         .map((layer3Dir) => `${layer2Dir}${layer3Dir}`)
  //         .forEach((layer3Dir) => {
  //           console.log(layer3Dir);
  //           const unityExePath = `${layer3Dir}/Editor/Unity.exe`;
  //           if (fs.existsSync(unityExePath)) {
  //             // 通过 -version 命令，获取版本信息
  //             const version = spawnSync(unityExePath, ['-version'], {
  //               encoding: 'utf8'
  //             }).stdout;
  //             console.log(`========= ${version} ${unityExePath}`);
  //             versions[version] = unityExePath;
  //           }
  //         });
  //     } catch (error) {}
  //   });
  return versions;
};

// 读取 Configs 配置
const readConfigs = (): GlobalConfig => {
  let data: GlobalConfig;
  try {
    if (!fs.existsSync(configFile)) {
      // 文件不存在时，先初始化配置数据
      data = initConfigs();
    } else {
      // 文件存在，读取配置文件内容
      const fileContents = fs.readFileSync(configFile, 'utf8');
      data = yaml.parse(fileContents) as GlobalConfig;
    }
  } catch (error) {
    console.error(error);
    // 在错误处理中，直接返回初始化的配置数据，而不写入文件
    data = initConfigs();
  }
  return data;
};

// 写入 Configs 配置
const writeConfigs = (configs: GlobalConfig) => {
  try {
    const configFile = `${userHomeDir}/.upmrc`;
    const yamlContent = yaml.stringify(configs);
    fs.writeFileSync(configFile, yamlContent, 'utf8');
  } catch (e) {
    console.error(e);
  }
};

export { initConfigs, scanEditorVersions, readConfigs, writeConfigs };
