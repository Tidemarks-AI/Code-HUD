import { Router } from 'express';
import axios from 'axios';
import type { Request, Response } from 'express';

export interface UserService {
  getUser(id: string): Promise<User>;
}

export class UserController {
  constructor(private service: UserService) {}

  async handleGet(req: Request, res: Response): Promise<void> {
    const user = await this.service.getUser(req.params.id);
    res.json(user);
  }
}

export function createRouter(): Router {
  return Router();
}
