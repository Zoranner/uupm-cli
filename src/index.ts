#!/usr/bin/env node

import * as cmd from 'commander';
import figlet from 'figlet';
import PackageResolver from './package-resolver.js';

const program = new cmd.Command('upm');

const sayHello = () => {
  console.log(
    figlet.textSync('_UPM_', {
      font: 'Ghost',
      horizontalLayout: 'default',
      verticalLayout: 'default',
      width: 80,
      whitespaceBreak: true
    })
  );
};

program
  .command('hello')
  .description('Say hello to nup!')
  .action(() => {
    sayHello();
  });

program
  .command('install')
  .alias('i')
  .description('Install a package.')
  .option('-N, --nuget', 'Install from NuGet package.')
  .argument('<name>', 'Package name to install.')
  .action((name, options) => {
    const resolver = new PackageResolver();
    console.log(options);
    if (options.nuget) {
      console.log(`Installing NuGet package: ${name}`);
      // 处理 NuGet 源的包
      resolver.recursionResolve(name);
    } else {
      console.log(`Installing package: ${name}`);
      // 处理 Unity 源的包
    }
  });

program.parse(process.argv);
