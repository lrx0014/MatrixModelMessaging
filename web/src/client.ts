import createClient from 'openapi-fetch';
import type { paths } from './generated/api';

// Same-origin health client; management authentication is not implemented in M0.
export const client = createClient<paths>();
export const liveness = () => client.GET('/health/live');
export const readiness = () => client.GET('/health/ready');
