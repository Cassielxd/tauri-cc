<template>
  <div id="app-base-httpserver">
    <div class="one-block-1">
      <span>
        1. 使用ipc与deno进程通信
      </span>
    </div>
    <div class="one-block-2">
      <p>* 发送请求：
        <a-button @click="sendIpcRequest('pictures')"> 发送请求 </a-button>
      </p>
      <p>* 结果：{{ result }}</p>
    </div>
    <div class="one-block-1">
      <span>
        2. 使用http与deno进程通信
      </span>
    </div>
    <div class="one-block-2">
      <p>
        <a-button @click="sendRequest()"> 发送请求 </a-button>
        （这里是后端deno http服务，请求地址为：http://localhost:9999/demo/test3333/hello）
      </p>
      <p>* 结果：{{ result }}</p>
    </div>
  </div>
</template>
<script>


import axios from 'axios';
import storage from 'store2';
import { denoManager } from './index';
let deno = null;
let listen1 =null;
let listen2 =null;
export default {
  data() {
    return {
      currentStatus: '关闭',
      servicAddress: '无',
      result:""
    };
  },
  mounted () {
    this.init1();
  },
  unmounted() {
    if(deno){
      deno.unlisten(listen1);
      deno.unlisten(listen2);
    }
  },
  methods: {
    async init1 () {
        deno = await denoManager.get("main");
      listen1=   deno.listenOn("test", (message) => {
          this.result = message;
          console.log('deno message0:', message);
        });
      listen2= await deno.listenOn("test1", (message) => {
          this.result = message;
          console.log('deno message1:', message);
        });
        console.log(deno);

    },
    sendIpcRequest () {
      deno.send('testIpc',{url:"http://localhost:9999/demo/test3333/hello",method:"POST",data:"aaaa"});
    },
    sendRequest (id) {
      this.requestHttp("demo/test3333/hello", {id}).then(res => {
        console.log('res:', res)
        this.result= {data:res.data,status:res.status};
      })
    },

    /**
     * Accessing built-in HTTP services
     */
    requestHttp(uri, parameter) {
      // URL conversion
      const host = 'http://localhost:9999';
      let url = uri;
      url = host + '/' + url;
      console.log('url:', url);
      return axios({
        url: url,
        method: 'post',
        data: parameter,
        timeout: 60000,
      })
    },

  }
};
</script>
<style lang="less" scoped>
#app-base-httpserver {
  padding: 0px 10px;
  text-align: left;
  width: 100%;
  .one-block-1 {
    font-size: 16px;
    padding-top: 10px;
  }
  .one-block-2 {
    padding-top: 10px;
  }
}
</style>
