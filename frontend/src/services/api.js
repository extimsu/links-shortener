/**
 * API Service Module
 *
 * - Uses VITE_API_BASE_URL for all requests
 * - Handles all backend endpoints
 * - Logs requests and responses
 * - Provides user-friendly error messages
 * - Retries network errors up to 2 times
 *
 * Usage:
 *   import { shortenUrl, getAnalytics } from './services/api';
 *   await shortenUrl('https://example.com');
 *   await getAnalytics('abc123');
 */
import axios from 'axios';

const apiClient = axios.create({
  baseURL: import.meta.env.VITE_API_BASE_URL || '',
  headers: {
    'Content-Type': 'application/json',
  },
});

// Request interceptor: log requests
apiClient.interceptors.request.use((config) => {
  // console.log('API Request:', config.method, config.url, config.data);
  return config;
});

// Response interceptor: log responses
apiClient.interceptors.response.use(
  (response) => {
    // console.log('API Response:', response.status, response.data);
    return response;
  },
  (error) => {
    // console.log('API Error:', error);
    return Promise.reject(error);
  }
);

async function withRetry(fn, retries = 2) {
  let lastErr;
  for (let i = 0; i <= retries; i++) {
    try {
      return await fn();
    } catch (err) {
      lastErr = err;
      // Only retry on network errors (no response)
      if (!err.response && i < retries) continue;
      break;
    }
  }
  throw lastErr;
}

export async function shortenUrl(url) {
  try {
    return await withRetry(async () => {
      const response = await apiClient.post('/api/shorten', { url });
      return response.data;
    });
  } catch (err) {
    let message = err.response?.data?.error || err.message || 'Failed to shorten URL.';
    if (!err.response) message = 'Network error: Unable to reach backend.';
    throw new Error(message);
  }
}

export async function getAnalytics(shortCode) {
  try {
    return await withRetry(async () => {
      const response = await apiClient.get(`/api/analytics/${shortCode}`);
      return response.data;
    });
  } catch (err) {
    let message = err.response?.data?.error || err.message || 'Failed to fetch analytics.';
    if (!err.response) message = 'Network error: Unable to reach backend.';
    throw new Error(message);
  }
}

export { apiClient };
