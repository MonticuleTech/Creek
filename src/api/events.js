import { listen } from '@tauri-apps/api/event';

export const onPong = async (callback) => {
  return await listen('pong', callback);
};

export const onDocumentUpdate = async (callback) => {
  return await listen('document-update', callback);
};

export const onLlmStream = async (callback) => {
  return await listen('llm-stream', callback);
};
