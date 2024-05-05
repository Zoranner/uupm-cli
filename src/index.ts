#!/usr/bin/env node

import * as cmd from 'commander';
import showVersion from './actions/show-version.js';
import showGraphic from './actions/show-graphic.js';
import {
  resolveNugetPackage,
  resolvePackage
} from './actions/resolve-package.js';

const command = new cmd.Command('upm');

command.option('-v, --version', 'output the version number.').action(() => {
  const options = command.opts();
  if (options.version) {
    showVersion();
  } else {
    showGraphic();
  }
});

command
  .command('install')
  .alias('i')
  .description('install a package.')
  .option('-n, --nuget', 'install package from nuget.')
  .argument('<name>', 'package name to install.')
  .action((name, options) => {
    if (!options.nuget) {
      console.log(`installing package: ${name}`);
      resolvePackage(name);
    } else {
      console.log(`installing package from NuGet: ${name}`);
      resolveNugetPackage(name);
    }
  });

program.parse(process.argv);
