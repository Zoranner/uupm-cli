import { Dependencies } from "./basic-struct.js";

interface PackageInfo {
  versions: {
    [version: string]: VersionDetail;
  };
  name: string;
}

interface VersionDetail {
  name: string;
  displayName: string;
  version: string;
  unity: string;
  description: string;
  dependencies: Dependencies;
  dist: Distribution;
}

interface Distribution {
  integrity: string;
  shasum: string;
  tarball: string;
}

export { PackageInfo, VersionDetail, Distribution };
