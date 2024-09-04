import BaseContextClass from "../../base.ts";
import { Controller, RequestMapping } from "../../core.ts";

@Controller("/test3333")
class Testtwo extends BaseContextClass {
  constructor(cxt) {
    super(cxt);
  }

  @RequestMapping("POST", "/hello")
  async hello(_req) {
    console.log(this.app);
    return "tetst";
  }

  @RequestMapping("POST", "/ssssssssss")
  async aaaaaaaaaa(_req) {
    return "tetst";
  }

  @RequestMapping("POST", "/aaaaaaaaa")
  async getaaaaa(_req) {
    return "tetst";
  }
}

export default Testtwo;
