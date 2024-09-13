// deno-lint-ignore-file
// deno-lint-ignore-file prefer-const
import { Args } from "./types.ts";
import config from "./deno.json" with  { type: "json" };
import { buildRouter } from "./core.ts";
import { Application } from "jsr:@oak/oak/application";
import { Router } from "jsr:@oak/oak/router";

class Context {
  service: { [key: string]: any } = {};
  controller: { [key: string]: any } = {};
  routers: Map<string, any> = new Map();
  urlPatterns: Map<string, URLPattern> = new Map();

  addService(name: string, service: any) {
    this.service[`${name}`] = service;
  }

  addController(name: string, controller: any) {
    this.controller[`${name}`] = controller;
  }

  addRouter(router: any) {
    for (const routerKey in router) {
      this.routers.set(routerKey, router[routerKey]);
    }
  }

  matchUrl(url: string) {
    for (let [key, value] of this.urlPatterns) {
      if (value.test(url)) {
        let match = value.exec(url);
        return { router: this.routers.get(key), groups: match?.pathname.groups };
      }
    }
    return null;
  }

  buildUrlPatterns() {
    for (let [key, value] of this.routers) {
      this.urlPatterns.set(key, new URLPattern({ pathname: key }));
    }
  }

  async loaderAndBuilder() {
    let { workspaces } = config;
    if (!workspaces) return;
    for (let i = 0; i < workspaces.length; i++) {
      try {
        let item = workspaces[i];
        let { controllers, services }:any = await import("./" + item + "/resource.ts");
        controllers.forEach(({ name, ClassName }:any) => {
          this.addRouter(buildRouter(item, name, ClassName));
          this.addController(name, new ClassName(this));
        });
        services.forEach(({ name, ClassName }:any) => {
          this.addService(name, new ClassName(this));
        });
      } catch (e) {
        console.log(e);
      }
    }
    this.buildUrlPatterns();
  }

  start() {
    const ctx = this;
    (async () => {
      await ctx.loaderAndBuilder();
       ctx.startIpcServer();
       ctx.startHttpServer();
    })();
   
  }
  async startIpcServer(){
    const ctx = this;
    const { controller } = ctx;
    let ipcBroadcastChannel = new IpcBroadcastChannel("testIpc");
    ipcBroadcastChannel.onmessage=async ({data:request}: MessageEvent)=>{
      let response = {status:200,message:"success",body:""};
      try {
        if(request.url){
          let url = new URL(request.url);
          const match = ctx.matchUrl(request.url);
          if (!match) throw new Error("notfound");
          if (match.router.method != request.method)
            throw new Error("not support " + request.method);
          const fn = controller[match.router.className][match.router.key];
          if (!fn) throw new Error("method notfound");
          response.body = await fn.call(
            controller[match.router.className], request
          );
        }else{
          response={status:500,message:"url is empty",body:""};
        }
      }catch (e:any) {
        response={status:500,message:e.message,body:""};
      }finally {
        console.log("ipc response")
        ipcBroadcastChannel.postMessage({key:"main",message:response});
      }   
    }
    //main：消息发送到主窗口的(如果为空 则发送到所有的窗口)  testIpc:事件名称(如果main窗口没有监听的话 是收不到的)
    
  }
  startHttpServer(){
    // deno-lint-ignore no-this-alias
    const self = this;
    const router = new Router();
    for (let [key, value] of this.routers) {
      switch (value.method) {
        case "POST":
          router.post(key, async (ctx) => {
            let body = await ctx.request.body.json();
           let responseBody = await self.controller[value.className][value.key](body);
           ctx.response.body = responseBody;
          });
          break;
        case "GET":
          router.get(key, async (ctx) => {
            let responseBody = await self.controller[value.className][value.key](ctx.params);
            ctx.response.body = responseBody;
           });
          break;
      }
    }
    
    const app = new Application();
    app.use(router.routes());
    app.use(router.allowedMethods());
    console.log("http://localhost:8080");
    app.listen({ port: 8080 });
  }
}

export default Context;