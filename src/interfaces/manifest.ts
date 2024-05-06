import { Dependencies } from "./basic-struct.js";

interface Manifest {
  dependencies: Dependencies;
  scopedRegistries: ScopedRegistry[];
}

interface ScopedRegistry {
  name: string;
  url: string;
  scopes: string[];
}

export { Manifest, ScopedRegistry };
