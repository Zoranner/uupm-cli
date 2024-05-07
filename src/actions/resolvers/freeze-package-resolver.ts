import axios from 'axios';
import * as fs from 'fs';
import * as path from 'path';
import fse from 'fs-extra';
import { Queue } from 'typescript-collections';
import { Manifest, ScopedRegistry } from '../../interfaces/package-manifest.js';
import { PackageInfo } from '../../interfaces/unity-package-info.js';

export default class FreezePackageResolver {
  private MAINFEST_PATH: string = 'Packages/manifest.json';
  private OFFICIAL_REGISTRY_URL = 'https://packages-v2.unity.com';

  private manifest?: Manifest;
  private packageQueue: Queue<[string, string]> = new Queue<[string, string]>();

  constructor() {}

  private async loadManifest(): Promise<Manifest | undefined> {
    if (!fs.existsSync(this.MAINFEST_PATH)) {
      return undefined;
    }
    const manifestContent = await fs.promises.readFile(
      this.MAINFEST_PATH,
      'utf8'
    );
    return JSON.parse(manifestContent);
  }

  async recursionResolve(): Promise<void> {
    this.manifest = await this.loadManifest();
    if (!this.manifest) {
      console.log(
        `No ${this.MAINFEST_PATH} file exists in the current directory.`
      );
      return;
    }

    for (const [packageName, packageVersion] of Object.entries(
      this.manifest.dependencies
    )) {
      // Skip already resolved packages
      if (packageVersion.startsWith('file:')) {
        continue;
      }
      this.packageQueue.enqueue([packageName, packageVersion]);
    }
    console.log();
    const scopedRegistries = this.manifest?.scopedRegistries;
    while (this.packageQueue.size() > 0) {
      const currentPackage = this.packageQueue.dequeue();
      if (!currentPackage || currentPackage.length != 2) {
        continue;
      }
      const [packageName, packageVersion] = currentPackage;
      console.log(`> ${packageName}@${packageVersion}`);
      const registryUrl = this.matchRegistryUrl(packageName, scopedRegistries);
      const freezeVersion = await this.singleResolve(
        packageName,
        packageVersion,
        registryUrl
      );
      if (freezeVersion) {
        this.manifest.dependencies[packageName] = freezeVersion;
      }
      console.log();
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
    if (packageName.startsWith('com.unity.modules')) {
      return null;
    }
    const packageInfoUrl = `${registryUrl}/${packageName}`;
    const response = await axios.get(packageInfoUrl);
    const packageInfo: PackageInfo = response.data;
    const versionInfo = packageInfo.versions[packageVersion];
    if (!versionInfo) {
      return;
    }
    // const packageUrl = `${registryUrl}/${packageName}/-/`;
    const downloadUrl = versionInfo.dist.tarball;
    const tarballName = `${packageName}-${packageVersion}.tgz`;
    await this.downloadPackage(downloadUrl, tarballName);
    const dependencies = versionInfo.dependencies;
    if (dependencies) {
      for (const [packageName, packageVersion] of Object.entries(
        dependencies
      )) {
        // Repeat the whole process for each dependency
        console.log(`  - ${packageName}@${packageVersion}`);
        this.packageQueue.enqueue([packageName, packageVersion]);
      }
    }
    return `file:${tarballName}`;
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
    return this.OFFICIAL_REGISTRY_URL;
  }

  private async downloadPackage(downloadUrl: string, fileName: string) {
    // console.log(`Downloading ${fileName} from ${downloadUrl}`);
    const response = await axios.get(downloadUrl, {
      responseType: 'arraybuffer'
    });
    const filePath = path.join('Packages', fileName);
    await fs.promises.writeFile(filePath, response.data);
  }
}
