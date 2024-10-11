import * as fs from 'fs';
import * as yaml from 'yaml';
import * as os from 'os';
import { spawnSync } from 'child_process';
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
          Nuget: 'https://api.nuget.org/v3/index.json'
        }
      }
    },
    editor: {
      default: '',
      version: {}
    }
  };
  initialConfigs.editor.version = scanEditorVersions();
  initialConfigs.editor.default = Object.keys(initialConfigs.editor.version)[0];
  return initialConfigs;
};

// 扫描已知的 Editor 版本
const scanEditorVersions = (): EditorVersions => {
  let versions: EditorVersions = {};
  editorBaseDirs.forEach((editorBaseDir) => {
    if (!fs.existsSync(editorBaseDir)) {
      return;
    }
    fs.readdirSync(editorBaseDir)
      .map((childDir) => `${editorBaseDir}${childDir}`)
      .forEach((childDir) => {
        const editorPath = `${childDir}/Editor`;
        const unityExePath = `${editorPath}/Unity.exe`;
        if (fs.existsSync(unityExePath)) {
          const version = spawnSync(
            'powershell',
            [
              '-command',
              `(Get-Item '${unityExePath}').VersionInfo.ProductVersion`
            ],
            {
              encoding: 'utf8',
              stdio: ['pipe', 'pipe', 'pipe']
            }
          ).stdout.split('_')[0];
          versions[version] = editorPath;
        }
      });
  });
  return versions;
};

// 读取 Configs 配置
const readConfigs = (): GlobalConfig => {
  let data: GlobalConfig;
  try {
    if (!fs.existsSync(configFile)) {
      // 文件不存在时，先初始化配置数据
      data = initConfigs();
      writeConfigs(data);
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
