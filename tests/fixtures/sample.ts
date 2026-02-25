export interface IUser {
  name: string;
  email: string;
}

export type UserId = string;

import { something } from './other';

export class UserService {
  getUser(id: string): IUser {
    return { name: 'test', email: 'test@test.com' };
  }
}

export enum Role {
  Admin = 'admin',
  User = 'user',
}

export function createUser(name: string): IUser {
  return { name, email: '' };
}
