import { useEffect, useState } from 'react';
import { getAnalytics } from '../services/api';
import './AnalyticsDisplay.css';

export default function AnalyticsDisplay({ shortCode }) {
  const [analytics, setAnalytics] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);

  const fetchAnalytics = async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await getAnalytics(shortCode);
      setAnalytics(data);
    } catch (err) {
      setError(err.message);
      setAnalytics(null);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (shortCode) fetchAnalytics();
    // eslint-disable-next-line
  }, [shortCode]);

  if (!shortCode) return null;

  return (
    <div className="analytics-container">
      <h2>Analytics</h2>
      <button className="refresh-btn" onClick={fetchAnalytics} disabled={loading}>
        {loading ? 'Refreshing...' : 'Refresh'}
      </button>
      {loading && <div className="loading">Loading analytics...</div>}
      {error && <div className="error">{error}</div>}
      {analytics && (
        <div className="analytics-info">
          <div className="analytics-row">
            <span>Short code:</span> <b>{analytics.short_code}</b>
          </div>
          <div className="analytics-row">
            <span>Original URL:</span> <span className="url">{analytics.original_url}</span>
          </div>
          <div className="analytics-row">
            <span>Created at:</span> <span>{new Date(analytics.created_at).toLocaleString()}</span>
          </div>
          <div className="analytics-row">
            <span>Transition count:</span>
            <span className="count-bar">
              <span className="bar" style={{ width: Math.min(analytics.transition_count * 10, 200) + 'px' }} />
              <b>{analytics.transition_count}</b>
            </span>
          </div>
        </div>
      )}
    </div>
  );
}
