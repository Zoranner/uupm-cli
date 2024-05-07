import { FreezePackageResolver } from './resolvers/index.js';

const freezePackage = () => {
  const resolver = new FreezePackageResolver();
  resolver.recursionResolve();
};

export { freezePackage };
