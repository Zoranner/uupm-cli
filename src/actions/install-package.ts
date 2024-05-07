import {
  NuGetPackageResolver,
  UnityPackageResolver
} from './resolvers/index.js';

const installPackage = (name: string) => {
  const resolver = new UnityPackageResolver();
  resolver.recursionResolve(name);
};

const installNugetPackage = (name: string) => {
  const resolver = new NuGetPackageResolver();
  resolver.recursionResolve(name);
};

export { installPackage, installNugetPackage };
