import Axios from 'axios';
import type { AxiosInstance, AxiosRequestConfig } from 'axios';

// Get the API base URL from Vite's environment variables.
// The VITE_ prefix is important for security.
const API_URL = import.meta.env.VITE_API_URL || 'http://localhost:8000';

export const AXIOS_INSTANCE = Axios.create({ 
  baseURL: API_URL,
  withCredentials: true,  // ✅ CRITICAL: Send cookies with every request
});

export const customInstance = <T>(
  config: AxiosRequestConfig,
  options?: AxiosRequestConfig,
): Promise<T> => {
  const source = Axios.CancelToken.source();
  const promise = AXIOS_INSTANCE({
    ...config,
    ...options, 
    cancelToken: source.token,
  }).then(({ data }) => data);

  // @ts-ignore
  promise.cancel = () => {
    source.cancel('Query was cancelled');
  };

  return promise;
};

