import BaseContextClass from "../../base.ts";
import {Controller,RequestMapping} from "../../core.ts";
@Controller("/test3333")
class Testtwo extends BaseContextClass {
  constructor(cxt) {
    super(cxt);
  }
  @RequestMapping("POST","/hello")
  async hello(_req) {
      console.log(this.app);

 /*     console.log(Object.getOwnPropertyNames(Test.prototype));
      const nodejs =Reflect.getOwnPropertyDescriptor(Test.prototype,"hello");
      console.log(nodejs.value);*/
/*    const nodejs =Reflect.getOwnPropertyDescriptor(Test.prototype,"hello");
      const nodejs2 =Reflect.getOwnPropertyDescriptor(Test.prototype,"constructor");
      console.log(nodejs2.value.prototype.path());
    console.log(nodejs.value.prototype.router());*/
    return "tetst";
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

export default Testtwo;
