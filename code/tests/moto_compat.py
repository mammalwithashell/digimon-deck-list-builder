"""Compatibility helpers for Moto-backed S3 tests."""

from __future__ import annotations

try:
    from moto import mock_aws
except ImportError:  # Moto 4.x exposes service-specific mocks.
    from moto import mock_s3 as mock_aws

__all__ = ["mock_aws"]
