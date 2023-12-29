import Context from "./context.ts";
class BaseContextClass {
  public service: any;
  public app: any;
  constructor(ctx: Context) {
    this.app = ctx;
    this.service = ctx.service;
  }
}

export default BaseContextClass;
