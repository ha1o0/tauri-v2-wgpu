import { invoke } from "@tauri-apps/api/core";
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';

let greetInputEl: HTMLInputElement | null;
let greetMsgEl: HTMLElement | null;
let newWindowButtonEl: HTMLElement | null;

async function greet() {
  if (greetMsgEl && greetInputEl) {
    // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
    greetMsgEl.textContent = await invoke("greet", {
      name: greetInputEl.value,
    });
  }
}

async function openNewWindow() {
  // 创建一个新窗口
  const webview = new WebviewWindow('second-window', {
    url: 'index.html',
    title: '新窗口',
    width: 800,
    height: 600,
  });
  
  // 监听窗口创建事件
  webview.once('tauri://created', () => {
    console.log('窗口已创建');
  });
  
  // 监听窗口错误事件
  webview.once('tauri://error', (e: any) => {
    console.error('窗口创建错误:', e);
  });
}

window.addEventListener("DOMContentLoaded", () => {
  greetInputEl = document.querySelector("#greet-input");
  greetMsgEl = document.querySelector("#greet-msg");
  newWindowButtonEl = document.querySelector("#new-window-button");
  
  document.querySelector("#greet-button")?.addEventListener("click", () => greet());
  
  newWindowButtonEl?.addEventListener("click", () => openNewWindow());
  
  greetInputEl?.addEventListener("keypress", (e) => {
    if (e.key === "Enter") {
      greet();
    }
  });
});
