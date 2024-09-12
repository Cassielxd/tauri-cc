import BaseContextClass from "../../base.ts";
import Context from "../../context.ts";
import { Controller, RequestMapping } from "../../core.ts";

@Controller("/test3333")
class Testtwo extends BaseContextClass {
  constructor(cxt: Context) {
    super(cxt);
  }

  @RequestMapping("POST", "/hello")
  async hello(_req: any) {
    console.log(this.app);
    return "tetst";
  }

  @RequestMapping("POST", "/ssssssssss")
  async aaaaaaaaaa(_req: any) {
    return "tetst";
  }

  @RequestMapping("POST", "/aaaaaaaaa")
  async getaaaaa(_req: any) {
    return "tetst";
  }
}

export default Testtwo;
