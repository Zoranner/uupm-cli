const installPackage = (name: string) => {
};

const installNugetPackage = (name: string) => {
  const resolver = new NuGetPackageResolver();
  resolver.recursionResolve(name);
};

export { installPackage, installNugetPackage };
