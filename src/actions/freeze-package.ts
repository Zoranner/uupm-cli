import { FreezePackageResolver } from './resolvers/index.js';

const freezePackage = async () => {
  console.log(`Freezing project packages...`);
  const resolver = new FreezePackageResolver();
  await resolver.recursionResolve();
  console.log(`Freeze finished!`);
};

export { freezePackage };
