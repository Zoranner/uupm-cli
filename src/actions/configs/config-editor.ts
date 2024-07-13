import { readConfigs, scanEditorVersions, writeConfigs } from './config-base.js';

const addEditor = (name: string, path: string) => {
  const configs = readConfigs();
  configs.editor.version[name] = path;
  writeConfigs(configs);
};

const removeEditor = (name: string) => {
  const configs = readConfigs();
  delete configs.editor.version[name];
  writeConfigs(configs);
};

const listEditors = () => {
  const versions = scanEditorVersions();
  console.log(versions);
  const configs = readConfigs();
  const editors = Object.keys(configs.editor.version);
  editors.forEach((editor) => {
    console.log(`${editor}: ${configs.editor.version[editor]}`);
  });
};

const setDefaultEditor = (name: string) => {
  const configs = readConfigs();
  if (!configs.editor.version[name]) {
    throw new Error(`${name} is not a editor version`);
  }
  configs.registry.origin.default = name;
  writeConfigs(configs);
};

export { addEditor, removeEditor, listEditors, setDefaultEditor };
