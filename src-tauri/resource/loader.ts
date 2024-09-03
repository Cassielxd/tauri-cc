import * as path from "https://deno.land/std@0.209.0/path/mod.ts";

/**
 * 自动加载指定目录下的文件，并将文件路径和文件名存入filePathArr数组中
 * @param directory - 目录路径
 * @param filePathArr - 存放文件路径和文件名的数组
 */
export const loader = async (worker: string, directory: string, basePath: string, filePathArr: any[]) => {
  for await (const dirEntry of Deno.readDir(directory)) {
    const filename = dirEntry.name;

    const filePath = path.join(directory, filename);
    const stats = Deno.statSync(filePath);
    if (stats.isDirectory) {
      await loader(worker, filePath, basePath, filePathArr);
    } else {
      const isFile = stats.isFile;
      const extname = isFile ? path.extname(filePath) : "";
      if (extname === ".js" || extname === ".ts") {
        const isService = filePath.includes("service");
        const isController = filePath.includes("controller");
        if (isService || isController) {
          let path = filePath.replace(basePath, "");
          path = "." + path;
          console.log(path);
          filePathArr.push({ worker, path, name: filename });
        }
      }
    }
  }
};