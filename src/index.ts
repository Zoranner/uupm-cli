#!/usr/bin/env node
import * as cmd from 'commander';
import showVersion from './actions/show-version.js';
import showGraphic from './actions/show-graphic.js';
import {
  installPackage,
  installNugetPackage
} from './actions/install-package.js';
import { freezePackage } from './actions/freeze-package.js';
import { addRegistry, listRegistries, removeRegistry } from './actions/manage-registry.js';

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
      installPackage(name);
    } else {
      installNugetPackage(name);
    }
  });

command
  .command('freeze')
  .alias('f')
  .action(() => {
    freezePackage();
  });
  
const scopeCommand = command.command('scope').description('Manage scopes.');

  scopeCommand
  .command('add')
  .alias('a')
  .description('add a new scope.')
  .argument('<name>', 'scope name to add.')
  .argument('<url>', 'scope url to add.')
  .action((name, url) => {
    addRegistry(name, url);
  });

scopeCommand
  .command('remove')
  .alias('r')
  .description('remove an existing scope.')
  .argument('<name>', 'scope name to remove.')
  .action((name) => {
    removeRegistry(name);
  });

scopeCommand
  .command('list')
  .alias('l')
  .description('list all scopes.')
  .action(() => {
    listRegistries();
  });

command.parse(process.argv);
