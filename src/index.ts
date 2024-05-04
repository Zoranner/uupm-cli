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
  .command('get')
  .description('Get a nuget package to unity package.')
  .argument('<name>', 'NuGet package name.')
  .action((name) => {
    const resolver = new PackageResolver();
    resolver.recursionResolve(name);
  });

program.parse(process.argv);
