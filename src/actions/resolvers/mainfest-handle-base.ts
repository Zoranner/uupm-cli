import * as fs from 'fs';
import { Manifest } from '../../interfaces/package-manifest.js';

export default class MainfestHandleBase {
  protected MAINFEST_PATH: string = 'Packages/manifest.json';
  protected OFFICIAL_REGISTRY_URL = 'https://packages-v2.unity.com';

  protected manifest: Manifest = { dependencies: {}, scopedRegistries: [] };

  constructor() {}

  protected async loadManifest(): Promise<Manifest | null> {
    if (!fs.existsSync(this.MAINFEST_PATH)) {
      return null;
    }
    const manifestContent = await fs.promises.readFile(
      this.MAINFEST_PATH,
      'utf8'
    );
    return JSON.parse(manifestContent);
  }
}
