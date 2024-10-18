import {
  NuGetPackageResolver,
  UnityPackageResolver
} from './resolvers/index.js';

const installPackage = async (name: string) => {
  console.log(`Installing package: ${name}...`);
  const resolver = new UnityPackageResolver();
  await resolver.recursionResolve(name);
  console.log(`Install finished!`);
};

const installNugetPackage = async (
  name: string,
  source: string | undefined = undefined
) => {
  console.log(`Installing package from NuGet: ${name}...`);
  const resolver = new NuGetPackageResolver();
  await resolver.recursionResolve(name, source);
  console.log(`Install finished!`);
};

export { installPackage, installNugetPackage };
