interface RegistrySources {
  [sourceName: string]: string;
}

interface Registry {
  origin: {
    default: string;
    source: RegistrySources;
  };
  nuget: {
    default: string;
    source: RegistrySources;
  };
}

interface EditorVersions {
  [version: string]: string;
}

interface Editor {
  default: string;
  version: EditorVersions;
}

interface GlobalConfig {
  registry: Registry;
  editor: Editor;
}

export { GlobalConfig, EditorVersions };
