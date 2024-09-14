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

import {invoke} from "@tauri-apps/api";
import { listen } from '@tauri-apps/api/event';
import axios from 'axios';
import storage from 'store2';

export default {
  data() {
    return {
      currentStatus: '关闭',
      servicAddress: '无',
      result:""
    };
  },
  mounted () {
    this.init();
  },
  methods: {
    init () {
      const unlisten =  listen('testIpc', (event) => {
          console.log(event);
          this.result= event.payload;
      })

    },
    sendIpcRequest () {
      invoke('plugin:ipcs|send_to_deno', {key:"main", name: 'testIpc', content: {url:"http://localhost:8080/demo/test3333/hello",method:"POST",data:"aaaa"} }).then((res) => {
        console.log(res);
      }).catch((err) => {
        console.log(err);
      });
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
