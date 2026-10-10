import { SQL } from 'bun';
import { env } from '../utils/env';

export const sql = new SQL(env.databaseUrl);
