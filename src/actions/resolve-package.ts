import { NuGetPackageResolver } from './resolvers/index.js';

const resolvePackage = (name: string) => {};

const resolveNugetPackage = (name: string) => {
  const resolver = new NuGetPackageResolver();
  resolver.recursionResolve(name);
};

export { resolvePackage, resolveNugetPackage };
