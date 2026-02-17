import { useState, useEffect } from 'react';
import { getCardImageUrl } from '@/utils/cardImages';

/**
 * Loads a card image directly from the digimoncard.io CDN.
 *
 * Note: We intentionally do NOT set `crossOrigin = 'anonymous'` because the
 * CDN doesn't return `Access-Control-Allow-Origin` headers.  Setting
 * crossOrigin would cause the browser to block the image entirely.  Without
 * it, the image loads fine in a normal <img> tag — we just can't draw it to
 * a canvas (which is an acceptable trade-off).
 */
export function useCardImage(cardId: string | null) {
  const [src, setSrc] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [hasError, setHasError] = useState(false);

  useEffect(() => {
    if (!cardId) {
      setSrc(null);
      setHasError(false);
      return;
    }

    const url = getCardImageUrl(cardId);
    setSrc(url);
    setIsLoading(true);
    setHasError(false);

    // Preload so we know when it's ready (or broken)
    const img = new Image();
    img.src = url;
    img.onload = () => setIsLoading(false);
    img.onerror = () => {
      setHasError(true);
      setIsLoading(false);
    };
  }, [cardId]);

  return { src, isLoading, hasError };
}
