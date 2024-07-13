import axios from 'axios';
import * as fs from 'fs';
import * as path from 'path';
import fse from 'fs-extra';
import { ScopedRegistry } from '../../interfaces/package-manifest.js';
import { PackageInfo } from '../../interfaces/unity-package-info.js';
import { loadStepSpinner } from './step-spinner.js';
import MainfestHandleBase from './mainfest-handle-base.js';

export default class FreezePackageResolver extends MainfestHandleBase {
  private packageMap: Map<string, string> = new Map<string, string>();

  constructor() {
    super();
  }

  async recursionResolve(): Promise<void> {
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
      const [packageName, packageVersion] = this.packageMap.entries().next().value;
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
            packageName.startsWith('com.unity.feature') ||
            packageName.startsWith('com.unity.2d.sprite')
          ) {
            return `Skipped: ${packageName}@${packageVersion}.`;
          }
          if (
            packageVersion.startsWith('file:') ||
            packageVersion.startsWith('git:')
          ) {
            return `Skipped: ${packageName}@${packageVersion}.`;
          }
          const packageInfoUrl = `${registryUrl}/${packageName}`;
          const response = await axios.get(packageInfoUrl);
          const packageInfo: PackageInfo = response.data;
          const versionInfo = packageInfo.versions[packageVersion];
          if (!versionInfo) {
            return `Skipped: ${packageName}@${packageVersion}.`;
          }
          const downloadUrl = versionInfo.dist.tarball;
          const tarballName = `${packageName}-${packageVersion}.tgz`;
          await this.downloadPackage(downloadUrl, tarballName);
          const dependencies = versionInfo.dependencies;
          if (dependencies) {
            for (const [depPackageName, depPackageVersion] of Object.entries(
              dependencies
            )) {
              this.addToPackageMap(depPackageName, depPackageVersion);
            }
          }
          this.manifest.dependencies[packageName] = `file:${tarballName}`;
          return `Frozen: ${packageName}@${packageVersion}.`;
        },
        errorAction: null,
        finallyAction: null
      }
    ]);
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