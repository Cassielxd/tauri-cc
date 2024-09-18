import { invoke, Channel } from "@tauri-apps/api/core";

export interface IpcMessage {
  name: string;
  rid: number;
  content: any;
}
export async function sendToDeno(value: IpcMessage): Promise<string | null> {
  return await invoke("plugin:deno|send_to_deno", {
    payload: value,
  }).then((r: any) => (r.value ? r.value : null));
}
export async function listenOn(
  rid: number,
  name: string
): Promise<string> {
  return await invoke("plugin:deno|listen_on", {
    payload: { rid, name },
  }).then((r: any) => (r));
}
export async function unlistenFrom(
  rid: number,
  listenerid:string,
  name: string
): Promise<string | null> {
  return await invoke("plugin:deno|unlisten_from", {
    payload: { rid, name,listenerid },
  }).then((r: any) => (r.value ? r.value : null));
}

export async function createDenoChannel(
  key: string,
  channel: Channel<any>
): Promise<number> {
  return await invoke("plugin:deno|create_deno_channel", {
    payload: { key, on_event: channel },
  }).then((r: any) => r);
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
  #map = new Map<string, string>();
  constructor(key: string) {
    super();
    this.#key = key;
    this.init();
  }
  //初始化DenoChannel
  async init() {
    this.#rid = await createDenoChannel(this.#key, this);
  }
  //向deno发送消息
  async send(name: string, value: any) {
    return await sendToDeno({ rid: this.#rid, name, content: value });
  }
 //监听
  async listenOn(name: string) {
    let uuid = await listenOn(this.#rid, name);
    if (this.#map.has(name)) {
       return;
    } else {
      this.#map.set(name, uuid||"");
    }
  }
  //解除监听
  async unlistenFrom(name: string) {
    let listenerid =this.#map.get(name);
    if(listenerid){
      await unlistenFrom(this.#rid, listenerid,name);
    }
    this.#map.delete(name);
  }
  //关闭
  async close(){
    //循环遍历map删除监听
    for (let [key, value] of this.#map) {
      await unlistenFrom(this.#rid, value,key);
    }
    await closeDenoChannel(this.#rid);
    this.#map.clear();
  }
}
