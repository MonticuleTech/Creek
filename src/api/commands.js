import { invoke } from '@tauri-apps/api/core';

export const greet = async (name) => {
  return await invoke('greet', { name });
};
