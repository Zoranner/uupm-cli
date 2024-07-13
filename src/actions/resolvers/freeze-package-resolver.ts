import axios from 'axios';
import * as fs from 'fs';
import * as path from 'path';
import fse from 'fs-extra';
import * as tar from 'tar';
import inquirer from 'inquirer';
import { ScopedRegistry } from '../../interfaces/package-manifest.js';
import {
  PackageInfo,
  VersionDetail
} from '../../interfaces/unity-package-info.js';
import { loadStepSpinner } from './step-spinner.js';
import MainfestHandleBase from './mainfest-handle-base.js';
import { readConfigs } from '../configs/config-base.js';
import { Dependencies } from '../../interfaces/basic-struct.js';

export default class FreezePackageResolver extends MainfestHandleBase {
  private BUILD_IN_PACKAGES = [
    'com.unity.2d.sprite',
    'com.unity.2d.tilemap',
    'com.unity.render-pipelines.core',
    'com.unity.render-pipelines.high-definition',
    'com.unity.shadergraph',
    'com.unity.rendering.denoising',
    'com.unity.ugui',
    'com.unity.render-pipelines.universal',
    'com.unity.visualeffectgraph'
  ];

  private editorPath = '';
  private packageMap: Map<string, string> = new Map<string, string>();

  constructor() {
    super();
  }

  async recursionResolve(): Promise<void> {
    this.editorPath = await this.selectUnityVersion();
    const manifest = await this.loadManifest();
    if (!manifest) {
      console.log(
        `No ${this.MAINFEST_PATH} file exists in the current directory.`
      );
      return;
    }
    this.manifest = manifest;
    for (const [packageName, packageVersion] of Object.entries(
      this.manifest.dependencies
    )) {
      this.addToPackageMap(packageName, packageVersion);
    }
    const scopedRegistries = this.manifest?.scopedRegistries;
    while (this.packageMap.size > 0) {
      const [packageName, packageVersion] = this.packageMap
        .entries()
        .next().value;
      this.packageMap.delete(packageName);
      const registryUrl = this.matchRegistryUrl(packageName, scopedRegistries);
      await this.singleResolve(packageName, packageVersion, registryUrl);
    }
    if (fs.existsSync(this.MAINFEST_PATH)) {
      await fse.copy(this.MAINFEST_PATH, 'Packages/manifest.src.json');
      await fse.remove(this.MAINFEST_PATH);
    }
    await fs.promises.writeFile(
      this.MAINFEST_PATH,
      JSON.stringify(this.manifest, null, 2),
      'utf8'
    );
  }

  private async singleResolve(
    packageName: string,
    packageVersion: string,
    registryUrl: string
  ) {
    await loadStepSpinner([
      {
        startTitle: 'Freezing unity package...',
        stepAction: async () => {
          if (
            packageName.startsWith('com.unity.modules') ||
            packageName.startsWith('com.unity.feature')
          ) {
            return `Skipped: ${packageName}@${packageVersion}.`;
          }

          if (
            packageVersion.startsWith('file:') ||
            packageVersion.startsWith('git:')
          ) {
            return `Skipped: ${packageName}@${packageVersion}.`;
          }

          let tarballName: string;
          let versionInfo: VersionDetail;

          if (this.BUILD_IN_PACKAGES.includes(packageName)) {
            const packagePath = path.join(
              this.editorPath,
              'Data',
              'Resources',
              'PackageManager',
              'BuiltInPackages',
              packageName
            );
            const packageFile = path.join(packagePath, 'package.json');
            if (!fs.existsSync(packageFile)) {
              return `Skipped: ${packageName}@${packageVersion}.`;
            }
            const packageContent = await fs.promises.readFile(
              packageFile,
              'utf8'
            );
            versionInfo = JSON.parse(packageContent);
            if (!versionInfo) {
              return `Skipped: ${packageName}@${packageVersion}.`;
            }
            // 复制包到 Packages 目录
            await fse.copy(packagePath, path.join('Packages', `${packageName}-${packageVersion}`));
            // 从 manifest.json 中删除包的 key
            delete this.manifest.dependencies[packageName];
          } else {
            const packageInfoUrl = `${registryUrl}/${packageName}`;
            const response = await axios.get(packageInfoUrl);
            const packageInfo: PackageInfo = response.data;
            versionInfo = packageInfo.versions[packageVersion];
            if (!versionInfo) {
              return `Skipped: ${packageName}@${packageVersion}.`;
            }
            tarballName = `${packageName}-${packageVersion}.tgz`;
            const downloadUrl = versionInfo.dist.tarball;
            await this.downloadPackage(downloadUrl, tarballName);
            this.manifest.dependencies[packageName] = `file:${tarballName}`;
          }

          this.resolveDependencies(versionInfo.dependencies);
          return `Frozen: ${packageName}@${packageVersion}.`;
        },
        errorAction: null,
        finallyAction: null
      }
    ]);
  }

  private async selectUnityVersion(): Promise<string> {
    const configs = readConfigs();
    const versions = Object.keys(configs.editor.version);
    const answers = await inquirer.prompt([
      {
        type: 'list',
        name: 'version',
        message: 'Please select the Unity version:',
        choices: versions
      }
    ]);
    return configs.editor.version[answers.version];
  }

  private async getBuiltInPackage(
    packageName: string,
    packageVersion: string
  ): Promise<string | null> {
    const packagePath = path.join(
      this.editorPath,
      'Data',
      'Resources',
      'PackageManager',
      'BuiltInPackages',
      packageName
    );
    const packageFile = path.join(packagePath, 'package.json');
    const tarballName = `${packageName}-${packageVersion}.tgz`;
    const tarballPath = path.join('Packages', tarballName);

    // 获取 build-in 包的依赖项
    if (!fs.existsSync(packageFile)) {
      return null;
    }
    const packageContent = await fs.promises.readFile(packageFile, 'utf8');
    const packageInfo: PackageInfo = JSON.parse(packageContent);
    const versionInfo = packageInfo.versions[packageVersion];
    if (!versionInfo) {
      return null;
    }
    const dependencies = versionInfo.dependencies;
    if (dependencies) {
      for (const [depPackageName, depPackageVersion] of Object.entries(
        dependencies
      )) {
        this.addToPackageMap(depPackageName, depPackageVersion);
      }
    }

    // 压缩为 .tgz 文件
    await tar.c(
      {
        gzip: true,
        file: tarballPath,
        cwd: packagePath
      },
      ['.']
    );

    return tarballName;
  }

  private async compressPackage(
    packagePath: string,
    tarballName: string
  ): Promise<void> {
    const tarballPath = path.join('Packages', tarballName);
    // 压缩为 .tgz 文件
    await tar.c(
      {
        gzip: true,
        file: tarballPath,
        cwd: packagePath
      },
      ['.']
    );
  }

  private matchRegistryUrl(
    packageName: string,
    scopedRegistries: ScopedRegistry[]
  ): string {
    for (const registry of scopedRegistries) {
      for (const scope of registry.scopes) {
        if (packageName.startsWith(scope)) {
          return registry.url.replace(/\/$/, ''); // Remove trailing slash if exists
        }
      }
    }
    return this.defaultRegistryUrl;
  }

  private async downloadPackage(downloadUrl: string, fileName: string) {
    const response = await axios.get(downloadUrl, {
      responseType: 'arraybuffer'
    });
    const filePath = path.join('Packages', fileName);
    await fs.promises.writeFile(filePath, response.data);
  }

  private resolveDependencies(dependencies: Dependencies) {
    if (dependencies) {
      for (const [depPackageName, depPackageVersion] of Object.entries(
        dependencies
      )) {
        this.addToPackageMap(depPackageName, depPackageVersion);
      }
    }
  }

  private addToPackageMap(packageName: string, packageVersion: string) {
    if (this.packageMap.has(packageName)) {
      const existingVersion = this.packageMap.get(packageName)!;
      if (this.compareVersions(packageVersion, existingVersion) > 0) {
        this.packageMap.set(packageName, packageVersion);
      }
    } else {
      this.packageMap.set(packageName, packageVersion);
    }
  }

  private compareVersions(version1: string, version2: string): number {
    const v1 = version1.split('.').map(Number);
    const v2 = version2.split('.').map(Number);
    for (let i = 0; i < Math.max(v1.length, v2.length); i++) {
      const num1 = v1[i] || 0;
      const num2 = v2[i] || 0;
      if (num1 > num2) return 1;
      if (num1 < num2) return -1;
    }
    return 0;
  }
}
