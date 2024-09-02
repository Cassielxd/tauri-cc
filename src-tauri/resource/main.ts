import  "./def.ts";
let worker = null;
globalThis.onload = async (e: Event): Promise<void> => {
  //用户worker 测试 一般会启动定时任务
  worker = new Worker(new URL("./worker.ts", import.meta.url).href, { type: "module" });
};


