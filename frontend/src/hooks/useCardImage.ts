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
export function useCardImage(cardId: string | null, altArt = false) {
  const [src, setSrc] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [hasError, setHasError] = useState(false);

  useEffect(() => {
    if (!cardId) {
      setSrc(null);
      setHasError(false);
      return;
    }

    const url = getCardImageUrl(cardId, altArt);
    setSrc(url);
    setIsLoading(true);
    setHasError(false);

    // Preload so we know when it's ready (or broken).
    // Note: we intentionally do NOT fall back from alt -> base on 404 here
    // because that would reproduce the "alt art showing as base art" bug the
    // grid dedupe is trying to eliminate.  Alt-art entries that the CDN
    // doesn't serve show the card's error state instead.
    const img = new Image();
    let cancelled = false;
    img.src = url;
    img.onload = () => {
      if (!cancelled) setIsLoading(false);
    };
    img.onerror = () => {
      if (cancelled) return;
      setHasError(true);
      setIsLoading(false);
    };
    return () => {
      cancelled = true;
    };
  }, [cardId, altArt]);

  return { src, isLoading, hasError };
}
