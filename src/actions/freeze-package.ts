import { readConfigs } from './configs/config-base.js';
import { FreezePackageResolver } from './resolvers/index.js';

const freezePackage = async () => {
  console.log(`Freezing project packages...`);
  const configs = readConfigs();
  console.log(JSON.stringify(configs));
  const registryName = configs.registry.origin.default;
  console.log(registryName);
  const registrySource = configs.registry.origin.source;
  console.log(registrySource);
  console.log(registrySource[registryName]);
  if (registrySource[registryName]) {

  }
  const resolver = new FreezePackageResolver();
  await resolver.recursionResolve();
  console.log(`Freeze finished!`);
};

export { freezePackage };
