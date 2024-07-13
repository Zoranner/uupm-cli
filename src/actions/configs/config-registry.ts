import { readConfigs, writeConfigs } from './config-base.js';

enum RegistryType {
  Origin = 'origin',
  Nuget = 'nuget'
}

const addRegistry = (
  name: string,
  url: string,
  type: RegistryType = RegistryType.Origin
) => {
  const configs = readConfigs();
  switch (type) {
    case RegistryType.Origin:
    default:
      configs.registry.origin.source[name] = url;
      break;
    case RegistryType.Nuget:
      configs.registry.nuget.source[name] = url;
      break;
  }
  writeConfigs(configs);
};

const removeRegistry = (
  name: string,
  type: RegistryType = RegistryType.Origin
) => {
  const configs = readConfigs();

  switch (type) {
    case RegistryType.Origin:
    default:
      delete configs.registry.origin.source[name];
      break;
    case RegistryType.Nuget:
      delete configs.registry.nuget.source[name];
      break;
  }
  writeConfigs(configs);
};

const listRegistries = (type: RegistryType = RegistryType.Origin) => {
  const configs = readConfigs();
  switch (type) {
    case RegistryType.Origin:
    default:
      console.log(configs.registry.origin.source);
      break;
    case RegistryType.Nuget:
      console.log(configs.registry.nuget.source);
      break;
  }
};

const setDefaultRegistry = (
  name: string,
  type: RegistryType = RegistryType.Origin
) => {
  const configs = readConfigs();
  switch (type) {
    case RegistryType.Origin:
    default:
      if (!configs.registry.origin.source[name]) {
        throw new Error(`${name} is not a registry`);
      }
      configs.registry.origin.default = name;
      break;
    case RegistryType.Nuget:
      if (!configs.registry.nuget.source[name]) {
        throw new Error(`${name} is not a nuget registry`);
      }
      configs.registry.nuget.default = name;
      break;
  }
  writeConfigs(configs);
};

export {
  RegistryType,
  addRegistry,
  removeRegistry,
  listRegistries,
  setDefaultRegistry
};
