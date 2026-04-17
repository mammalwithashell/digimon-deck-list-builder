"""DigitalOcean Spaces (S3-compatible) `ModelStorage`.

`public_url` returns the CDN URL so desktop clients download directly
from the Spaces edge — the Droplet running the API never sees the blob
bytes on the read path. Uploads go through the API over the admin
endpoint and are streamed into Spaces via `aioboto3`.
"""

from __future__ import annotations

from pathlib import Path
from typing import Optional


class SpacesModelStorage:
    def __init__(
        self,
        *,
        endpoint_url: str,
        region: str,
        bucket: str,
        access_key: str,
        secret_key: str,
        cdn_base_url: Optional[str] = None,
    ) -> None:
        if not all([endpoint_url, region, bucket, access_key, secret_key]):
            raise ValueError("SpacesModelStorage requires endpoint_url, region, bucket, access_key, secret_key")
        self.endpoint_url = endpoint_url.rstrip("/")
        self.region = region
        self.bucket = bucket
        self.access_key = access_key
        self.secret_key = secret_key
        # If cdn_base_url isn't set explicitly, derive the CDN hostname
        # from the endpoint (bucket.region.cdn.digitaloceanspaces.com is
        # the DO convention when Spaces CDN is enabled).
        self.cdn_base_url = (cdn_base_url or f"{self._default_cdn_base()}").rstrip("/")

    def _default_cdn_base(self) -> str:
        # endpoint_url like https://nyc3.digitaloceanspaces.com →
        # https://{bucket}.nyc3.cdn.digitaloceanspaces.com
        host = self.endpoint_url.split("://", 1)[-1]
        return f"https://{self.bucket}.{host.replace('.digitaloceanspaces.com', '.cdn.digitaloceanspaces.com')}"

    def _session(self):
        import aioboto3

        return aioboto3.Session(
            aws_access_key_id=self.access_key,
            aws_secret_access_key=self.secret_key,
            region_name=self.region,
        )

    def _client_kwargs(self) -> dict:
        return {"endpoint_url": self.endpoint_url, "region_name": self.region}

    async def put(self, key: str, source_path: Path, content_type: str) -> None:
        session = self._session()
        async with session.client("s3", **self._client_kwargs()) as s3:
            # upload_file handles multipart chunking for large blobs; the
            # default threshold (8 MiB) is fine for the 100–300 MB ONNX
            # files we ship.
            await s3.upload_file(
                str(source_path),
                self.bucket,
                key,
                ExtraArgs={"ContentType": content_type, "ACL": "public-read"},
            )

    async def delete(self, key: str) -> None:
        session = self._session()
        async with session.client("s3", **self._client_kwargs()) as s3:
            await s3.delete_object(Bucket=self.bucket, Key=key)

    async def exists(self, key: str) -> bool:
        session = self._session()
        async with session.client("s3", **self._client_kwargs()) as s3:
            try:
                await s3.head_object(Bucket=self.bucket, Key=key)
                return True
            except Exception:
                # botocore raises ClientError(404) on missing keys; other
                # errors (credentials, network) also end up here. Treat
                # all non-200 as "doesn't exist" — the caller will retry
                # write operations and get a proper error from put().
                return False

    def public_url(self, key: str) -> Optional[str]:
        return f"{self.cdn_base_url}/{key.lstrip('/')}"

    def local_path(self, key: str) -> Optional[Path]:
        return None
