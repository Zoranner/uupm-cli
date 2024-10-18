#!/usr/bin/env node
import * as cmd from 'commander';
import showVersion from './actions/show-version.js';
import showGraphic from './actions/show-graphic.js';
import {
  installPackage,
  installNugetPackage
} from './actions/install-package.js';
import { freezePackage } from './actions/freeze-package.js';
import {
  RegistryType,
  addRegistry,
  listRegistries,
  removeRegistry
} from './actions/configs/config-registry.js';
import {
  addEditor,
  listEditors,
  removeEditor
} from './actions/configs/config-editor.js';

const command = new cmd.Command('uupm');

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
  .option('-s, --source', 'install package from source.')
  .argument('[source]', 'nuget package source name.')
  .action((name, source, options) => {
    if (!options.nuget) {
      installPackage(name);
    } else {
      if (!options.source) {
      installNugetPackage(name);
      } else {
        installNugetPackage(name, source);
      }
    }
  });

command
  .command('freeze')
  .alias('f')
  .action(() => {
    freezePackage();
  });

const registryCommand = command
  .command('registry')
  .description('Manage registries.');
registryCommand
  .command('add')
  .alias('a')
  .option('-n, --nuget', 'registry for nuget.')
  .description('add a new registry.')
  .argument('<name>', 'registry name to add.')
  .argument('<url>', 'registry url to add.')
  .action((name, url, options) => {
    if (!options.nuget) {
      addRegistry(name, url);
    } else {
      addRegistry(name, url, RegistryType.Nuget);
    }
  });

registryCommand
  .command('remove')
  .alias('r')
  .option('-n, --nuget', 'registry for nuget.')
  .description('remove an existing registry.')
  .argument('<name>', 'registry name to remove.')
  .action((name, options) => {
    if (!options.nuget) {
      removeRegistry(name);
    } else {
      removeRegistry(name, RegistryType.Nuget);
    }
  });

registryCommand
  .command('list')
  .alias('l')
  .option('-n, --nuget', 'registry for nuget.')
  .description('list all registries.')
  .action((options) => {
    if (!options.nuget) {
      listRegistries();
    } else {
      listRegistries(RegistryType.Nuget);
    }
  });

const editorCommand = command.command('editor').description('Manage editors.');
editorCommand
  .command('scan')
  .alias('s')
  .description('scan current editor.')
  .action((name, url) => {
    addRegistry(name, url);
  });

editorCommand
  .command('add')
  .alias('a')
  .description('add a new editor.')
  .argument('<name>', 'editor name to add.')
  .argument('<path>', 'editor path to add.')
  .action((name, path) => {
    addEditor(name, path);
  });

editorCommand
  .command('remove')
  .alias('r')
  .description('remove an existing editor.')
  .argument('<name>', 'editor name to remove.')
  .action((name) => {
    removeEditor(name);
  });

editorCommand
  .command('list')
  .alias('l')
  .description('list all editors.')
  .action(() => {
    listEditors();
  });

command.parse(process.argv);
