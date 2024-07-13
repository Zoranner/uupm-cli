import axios from 'axios';
import * as fs from 'fs';
import * as path from 'path';
import fse from 'fs-extra';
import { Queue } from 'typescript-collections';
import { ScopedRegistry } from '../../interfaces/package-manifest.js';
import { PackageInfo } from '../../interfaces/unity-package-info.js';
import { loadStepSpinner } from './step-spinner.js';
import MainfestHandleBase from './mainfest-handle-base.js';

export default class FreezePackageResolver extends MainfestHandleBase {
  private packageQueue: Queue<[string, string]> = new Queue<[string, string]>();

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
      this.packageQueue.enqueue([packageName, packageVersion]);
    }
    const scopedRegistries = this.manifest?.scopedRegistries;
    while (this.packageQueue.size() > 0) {
      const currentPackage = this.packageQueue.dequeue();
      if (!currentPackage || currentPackage.length != 2) {
        continue;
      }
      const [packageName, packageVersion] = currentPackage;
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
            for (const [packageName, packageVersion] of Object.entries(
              dependencies
            )) {
              // console.log(`  - ${packageName}@${packageVersion}`);
              this.packageQueue.enqueue([packageName, packageVersion]);
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
}
