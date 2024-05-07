import fs from 'fs';
import path from 'path';
import os from 'os';
import toml from '@iarna/toml';

const upmrcPath = path.join(os.homedir(), '.upmrc');

// 读取 registries 配置
const readRegistries = () => {};

// 写入 registries 配置
const writeRegistries = (registries: Registry[]) => {};

const addRegistry = (name?: string, url?: string) => {};

const removeRegistry = (name?: string) => {};

const listRegistries = () => {};

const setDefaultRegistry = () => {};

export { addRegistry, removeRegistry, listRegistries };
