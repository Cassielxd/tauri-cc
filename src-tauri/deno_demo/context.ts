import { Args } from "./types.ts";
import config from "./deno.json" with  { type: "json" };
import { buildRouter } from "./core.ts";

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
        let { controllers, services } = await import("./" + item + "/resource.ts");
        controllers.forEach(({ name, ClassName }) => {
          this.addRouter(buildRouter(item, name, ClassName));
          this.addController(name, new ClassName(this));
        });
        services.forEach(({ name, ClassName }) => {
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
      const { controller } = ctx;
      const httpconn = new IpcConn("testIpc");
      for await (const { request, respondWith } of httpconn) {
        try {
             console.log(request);
        } catch (e) {
      
        } finally {
  
        }
      }
    })();
  }
}

export default Context;