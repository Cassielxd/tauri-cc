<template>
  <div id="app-base-socket-ipc">
    <div class="one-block-1">
      <span>
        异步消息
      </span>
    </div>
    <div class="one-block-2">
      <a-space>
        <a-button @click="handleInvoke">发送 - 回调</a-button>
        结果：{{ message1 }}
      </a-space>
      <p></p>
      <a-space>
        <a-button @click="handleInvoke2">发送 - async/await</a-button>
        结果：{{ message2 }}
      </a-space>
    </div>
    <div class="one-block-1">
      <span>
        窗口
      </span>
    </div>
    <div class="one-block-2">
      <a-space>
        <a-button @click="createWindow">创建window</a-button>

      </a-space>
    </div>
  </div>
</template>
<script>

import { toRaw } from 'vue';
import { invoke } from "@tauri-apps/api/core";
import { Window } from "@tauri-apps/api/window"
import { Webview } from "@tauri-apps/api/webview"
export default {
  data() {
    return {
      messageString: '',
      message1: '',
      message2: '',
      message3: '',
      windowName: 'window-ipc',
      newWcId: 0,
      views: [
        {
          type: 'vue',
          content: '#/special/subwindow',
          windowName: 'window-ipc',
          windowTitle: 'ipc window'
        },
      ],
    }
  },
  mounted () {
    this.init();

  },
  methods: {
    init () {

    },
    sendMsgStart() {
      const params = {
        type: 'start',
        content: '所有窗口'
      }
      emit('click', params)
    },
    sendMsgStart2() {
      const params = {
        type: 'start',
        content: '当前窗口'
      }
      appWindow.emit('click', params)
    },

    handleInvoke() {
      invoke("async_message", {value:"asdasdsadsa"}).then(r => {
        console.log('r:', r);
        this.message1 = r;
      });
    },
    async handleInvoke2() {
      const msg = await invoke("async_message", {value:"asdasdsadsa"});
      console.log('msg:', msg);
      this.message2 = msg;
    },
    handleSendSync() {
      const msg = invoke("sync_message", {invoke_message:"asdasdsadsa"});
      this.message3 = msg;
    },
    createWindow() {
      const appWindow = new Window('main');
      const webview = new Webview(appWindow, 'theUniqueLabel', {
        url: 'https://github.com/tauri-apps/tauri',
        x:400,
        y:800,
        width: 200,
        height: 200
      });
      const webview1 = new Webview(appWindow, 'theUniqueLabel1', {
        url: 'https://www.baidu.com',
        x:800,
        y:100,
        width: 200,
        height: 200
      });
      webview.once('tauri://created', function () {
        // webview successfully created
      });
      webview.once('tauri://error', function (e) {
        // an error happened creating the webview
        console.log(e);
      });
    },
    async sendTosubWindow() {
      // 新窗口id
    },
  }
}
</script>
<style lang="less" scoped>
#app-base-socket-ipc {
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
