import BaseContextClass from "../../base.ts";
import {Controller,RequestMapping} from "../../core.ts";
import {Data} from "../../model/data.ts";

@Controller("/test")
class Test extends BaseContextClass {
  constructor(cxt) {
    super(cxt);
  }
  @RequestMapping("POST","/hello/:id/:name")
  async hello({request,pathVariable:{id,name},params}) {
    const da = await Data.all();
    return JSON.stringify(da);
  }
    @RequestMapping("POST","/ssssssssss")
    async aaaaaaaaaa(_req) {
        return "tetst";
    }
    @RequestMapping("POST","/aaaaaaaaa")
    async getaaaaa(_req) {
        return "tetst";
    }
}

export default Test;
