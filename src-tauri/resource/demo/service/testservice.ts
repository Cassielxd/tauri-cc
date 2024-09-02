import BaseContextClass from "../../base.ts";

class TestService extends BaseContextClass {
  constructor(ctx) {
    super(ctx);
  }

  async hello() {
    return "hello services";
  }
}

export default TestService;
