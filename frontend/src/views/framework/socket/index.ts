import { invoke, Channel } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
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
export async function listenOn(rid: number, name: string): Promise<string> {
  return await invoke("plugin:deno|listen_on", { rid, name }).then(
    (r: any) => r
  );
}
export async function unlistenFrom(
  rid: number,
  name: string
): Promise<string | null> {
  return await invoke("plugin:deno|unlisten_from", { rid, name }).then(
    (r: any) => (r.value ? r.value : null)
  );
}

export async function createDenoChannel(
  key: string,
  channel: Channel<any>
): Promise<number> {
  return await invoke("plugin:deno|create_deno_channel", {
    key: key,
    onEvent: channel,
  });
}
export async function closeDenoChannel(rid: number): Promise<string> {
  return await invoke("plugin:deno|close_deno_channel", {
    payload: { rid },
  }).then((r: any) => r);
}
export async function checkDenoChannel(
  key: string
): Promise<boolean> {
  return await invoke("plugin:deno|check_deno_channel", {
    key: key,
  });
}
export async function cleanDenoChannel(
): Promise<boolean> {
  return await invoke("plugin:deno|clean_deno_channel", {});
}

interface ChannelMessage {
  event: String; //对应的事件
  content: any;
}
interface Litype {
  name: String; //对应的事件
  fn: any;
  id:number;
}
const manager: Map<string, Deno>= new Map();
let task:any= null;
class DenoManager{
  constructor(){
      if(!manager.size){
       console.log("Clear zombie channel")
        cleanDenoChannel()
      }
  }
   async get(key: string): Promise<Deno|undefined> {
    if(manager.has(key)){
      return manager.get(key);
    }
    let deno = await Deno.create(key);
    if(!deno.rid){
      console.log("deno instance undefined");
      return undefined;
    }
    manager.set(key, deno);
    return deno;
  }
   async close(key: string) {
    let deno = manager.get(key);
    if(deno){
      await deno.close();
      manager.delete(key);
    }
  }
   async closeAll() {
    for(let [key, value] of manager.entries()){
      await value.close();
      manager.delete(key);
    }
  }
}
getCurrentWebviewWindow().onCloseRequested(async (event) => {
   await cleanDenoChannel();
   console.log("denoManager closeAll");
});
export const denoManager = new DenoManager();
//deno channe默认实现 主要用于后端的 deno服务的通信
 class Deno extends Channel<ChannelMessage> {
  #key: string;
  #rid: number = 0;
  #status: "start" | "run" | "close";
  arr: Litype[] = [];
  static async create(key: string): Promise<Deno> {
    let deno = new Deno(key);
    await deno.init();
    return deno;
  }
  constructor(key: string) {
    super();
    this.#key = key;
    this.#status = "start";
    this.onmessage = (data) => {
      this.arr.forEach((item: any) => {
        if (item.name == data.event) {
          item.fn(data.content);
        }
      });
    };
  }
  get rid(){
    return this.#rid;
  }
  //初始化DenoChannel
  async init(fn?: any) {
    if (this.#status == "start") {
      this.#rid = await createDenoChannel(this.#key, this);
      this.#status = "run";
      if (fn) {
        await fn();
      }
    }
  }
  //向deno发送消息
  async send(name: string, value: any) {
    if (this.#status == "close") {
      console.log("deno channel is closed");
      return;
    }
    return await sendToDeno({ rid: this.#rid, name, content: value });
  }
  //监听
  async listenOn(name: string, fn: any) {
    if (this.#status == "close") {
      console.log("deno channel is closed");
      return;
    }
    await listenOn(this.#rid, name);
    let id = new Date().getTime();
    this.arr.push({ name, fn ,id });
    return ()=>{this.arr=this.arr.filter(item=>item.id!=id);};
  }
  //解除监听
  async unlistenFrom(name: string) {
    if (this.#status == "close") {
      console.log("deno channel is closed");
      return;
    }
    await unlistenFrom(this.#rid, name);
  }
  //关闭
  async close() {
    await closeDenoChannel(this.#rid);
    this.#status = "close";
  }
}
