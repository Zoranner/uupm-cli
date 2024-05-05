#!/usr/bin/env node

import * as cmd from 'commander';
import figlet from 'figlet';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import NuGetPackageResolver from './actions/resolvers/nuget-package-resolver.js';

const program = new cmd.Command('upm');
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const showGraphic = () => {
  console.log();
  console.log(
    figlet.textSync('_UPM_', {
      font: 'Ghost',
      horizontalLayout: 'default',
      verticalLayout: 'default',
      whitespaceBreak: true
    })
  );
  console.log();
};

const showVersion = () => {
  const packagePath = path.join(__dirname, '../package.json');
  const packageJson = JSON.parse(fs.readFileSync(packagePath, 'utf8'));
  console.log(`Version: ${packageJson.version}`);
};

program.option('-v, --version', 'Output the version number.').action(() => {
  const options = program.opts();
  if (options.version) {
    showVersion();
  } else {
    showGraphic();
  }
});

program
  .command('install')
  .alias('i')
  .description('Install a package.')
  .option('-n, --nuget', 'Install package from NuGet.')
  .argument('<name>', 'Package name to install.')
  .action((name, options) => {
    const resolver = new NuGetPackageResolver();
    if (options.nuget) {
      console.log(`Installing package from NuGet: ${name}`);
      // 处理 NuGet 源的包
      resolver.recursionResolve(name);
    } else {
      console.log(`Installing package: ${name}`);
      // 处理 Unity 源的包
    }
  });

program.parse(process.argv);
