console.log("worker 启动成功");

self.onmessage = async (e) => {
    self.close();
};