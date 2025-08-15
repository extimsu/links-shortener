import { useState } from 'react';
import { shortenUrl } from './services/api';
import AnalyticsDisplay from './components/AnalyticsDisplay';
import './App.css';

function App() {
  const [url, setUrl] = useState('');
  const [shortUrl, setShortUrl] = useState(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);
  const [copied, setCopied] = useState(false);

  const validateUrl = (value) => {
    try {
      const parsed = new URL(value);
      return parsed.protocol === 'http:' || parsed.protocol === 'https:';
    } catch {
      return false;
    }
  };

  const handleSubmit = async (e) => {
    e.preventDefault();
    setError(null);
    setShortUrl(null);
    setCopied(false);
    if (!validateUrl(url)) {
      setError('Please enter a valid http(s) URL.');
      return;
    }
    setLoading(true);
    try {
      const data = await shortenUrl(url);
      setShortUrl(data.short_url);
    } catch (err) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  const handleCopy = async () => {
    if (shortUrl) {
      await navigator.clipboard.writeText(shortUrl);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    }
  };

  // Extract short code from the shortUrl (assume last path segment)
  const shortCode = shortUrl ? shortUrl.split('/').pop() : null;

  return (
    <div className="container">
      <h1>URL Shortener</h1>
      <form className="shorten-form" onSubmit={handleSubmit}>
        <input
          type="url"
          placeholder="Enter a long URL..."
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          required
        />
        <button type="submit" disabled={loading}>
          {loading ? 'Shortening...' : 'Shorten URL'}
        </button>
      </form>
      {error && <div className="error">{error}</div>}
      {shortUrl && (
        <div className="result">
          <span className="short-url">{shortUrl}</span>
          <button className="copy-btn" onClick={handleCopy}>
            {copied ? 'Copied!' : 'Copy'}
          </button>
        </div>
      )}
      {/* Render analytics if a short code is available */}
      {shortCode && <AnalyticsDisplay shortCode={shortCode} />}
    </div>
  );
}

export default App;
