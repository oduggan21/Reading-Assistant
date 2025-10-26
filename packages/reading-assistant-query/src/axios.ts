import Axios from 'axios';
import type { AxiosInstance, AxiosRequestConfig } from 'axios';

// Get the API base URL from Vite's environment variables.
// The VITE_ prefix is important for security.
const API_URL = import.meta.env.VITE_API_URL || 'http://localhost:8000';

export const AXIOS_INSTANCE = Axios.create({ baseURL: API_URL });

// This is the function that our Orval-generated hooks will use.
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

  // You can add cancellation logic here if needed by your application
  // (e.g., in a useEffect cleanup function).
  // @ts-ignore
  promise.cancel = () => {
    source.cancel('Query was cancelled');
  };

  return promise;
};


