#!/usr/bin/env node

import * as cmd from 'commander';
import showVersion from './actions/show-version.js';
import showGraphic from './actions/show-graphic.js';
import {
  installPackage,
  installNugetPackage
} from './actions/install-package.js';

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
      console.log(`Installing package: ${name}`);
      installPackage(name);
    } else {
      console.log(`Installing package from NuGet: ${name}`);
      installNugetPackage(name);
    }
  });


command.parse(process.argv);
