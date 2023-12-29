import { Database, SQLite3Connector, Model, DataTypes } from "./orm/mod.ts";
import { Data } from "./model/data.ts";

const connector = new SQLite3Connector({
  filepath: "local_storage.sqlite"
});

const db = new Database(connector);
db.link([Data]);
db.sync();
export default db;
/*import mysql from "npm:mysql2@^2.3.3/promise";
const connection = await mysql.createConnection({
    host: "localhost",
    database:"denos",
    user: "root",
    password: "123456",
});

const  [results, fields] = await connection.query("SELECT * FROM `dinosaurs`");
console.log(results);
console.log(fields.toString());*/
/*
import { Database } from "https://deno.land/x/sqlite3@0.10.0/mod.ts";

const orm = new Database("local_storage");

const value = orm.prepare("select * from  data").value();
console.log(value);
*/
