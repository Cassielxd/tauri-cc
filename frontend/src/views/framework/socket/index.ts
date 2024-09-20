import { invoke, Channel } from "@tauri-apps/api/core";

export interface IpcMessage {
  name: string;
  rid: number;
  content: any;
}
export async function sendToDeno(value: IpcMessage): Promise<string | null> {
  return await invoke("plugin:deno|send_to_deno", {
    ...value,
  });
}
export async function listenOn(
  rid: number,
  name: string
): Promise<string> {
  return await invoke("plugin:deno|listen_on", {rid, name}).then((r: any) => (r));
}
export async function unlistenFrom(
  rid: number,
  name: string
): Promise<string | null> {
  return await invoke("plugin:deno|unlisten_from", { rid, name }).then((r: any) => (r.value ? r.value : null));
}

export async function createDenoChannel(
  key: string,
  channel: Channel<any>
): Promise<number> {
  return await invoke("plugin:deno|create_deno_channel", { key:key, onEvent: channel });
}
export async function closeDenoChannel(
  rid: number
): Promise<string> {
  return await invoke("plugin:deno|close_deno_channel", {
    payload: { rid },
  }).then((r: any) => (r));}


//deno channe默认实现 主要用于后端的 deno服务的通信
export class Deno extends Channel<any> {
  #key: string;
  #rid: number = 0;
  #status: "start"|"run"|"close"
  constructor(key: string) {
    super();
    this.#key = key;
    this.#status = "start";
  }
  //初始化DenoChannel
  async init(fn?:any) {
    if(this.#status=="start"){
      this.#rid = await createDenoChannel(this.#key, this);
      this.#status = "run";
      if(fn){
        await fn();
      }
    }
  }
  //向deno发送消息
  async send(name: string, value: any) {
    return await sendToDeno({ rid: this.#rid, name, content: value });
  }
  //监听
  async listenOn(name: string) {
    await listenOn(this.#rid, name);
  }
  //解除监听
  async unlistenFrom(name: string) {
    await unlistenFrom(this.#rid,name);
  }
  //关闭
  async close(){
    await closeDenoChannel(this.#rid);
    this.#status = "close";
  }
}
