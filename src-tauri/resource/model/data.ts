import {Model,DataTypes  } from '../orm/mod.ts'

export class Data extends Model {
    static table = 'data';
    static fields = {
        key: {primaryKey: true,type:DataTypes.STRING},
        value: DataTypes.STRING,
    };
}