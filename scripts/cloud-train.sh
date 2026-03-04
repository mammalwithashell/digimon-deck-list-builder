#!/usr/bin/env bash
# cloud-train.sh — Provision a cloud GPU VM and run the training stack.
#
# Supports GCP and AWS. Requires gcloud or aws CLI to be configured.
#
# Usage:
#   ./scripts/cloud-train.sh gcp [--gpus 4] [--spot]
#   ./scripts/cloud-train.sh aws [--gpus 4] [--spot]
#   ./scripts/cloud-train.sh ssh            # SSH into existing VM
#   ./scripts/cloud-train.sh teardown       # Delete the VM
#
# Prerequisites:
#   GCP: gcloud CLI authenticated, project set, GPU quota approved
#   AWS: aws CLI authenticated, default region set

set -euo pipefail

VM_NAME="digimon-training"
NUM_GPUS="${NUM_GPUS:-1}"
USE_SPOT=false
MAX_CONCURRENT="${MAX_CONCURRENT:-4}"

# ── Parse args ─────────────────────────────────────────────────────────

PROVIDER="${1:-help}"
shift || true

while [[ $# -gt 0 ]]; do
    case "$1" in
        --gpus) NUM_GPUS="$2"; shift 2 ;;
        --spot) USE_SPOT=true; shift ;;
        --concurrent) MAX_CONCURRENT="$2"; shift 2 ;;
        *) echo "Unknown arg: $1"; exit 1 ;;
    esac
done

# ── GCP ────────────────────────────────────────────────────────────────

gcp_create() {
    local ZONE="us-central1-a"
    local MACHINE_TYPE="g2-standard-8"
    local ACCELERATOR="type=nvidia-l4,count=${NUM_GPUS}"

    if [[ "$NUM_GPUS" -gt 1 ]]; then
        # Scale machine type with GPU count
        local VCPUS=$((NUM_GPUS * 12))
        MACHINE_TYPE="g2-standard-${VCPUS}"
    fi

    local SPOT_FLAG=""
    if $USE_SPOT; then
        SPOT_FLAG="--provisioning-model=SPOT --instance-termination-action=STOP"
    fi

    echo "Creating GCP VM: $VM_NAME ($MACHINE_TYPE, ${NUM_GPUS}x L4 GPU, spot=$USE_SPOT)"

    gcloud compute instances create "$VM_NAME" \
        --zone="$ZONE" \
        --machine-type="$MACHINE_TYPE" \
        --accelerator="$ACCELERATOR" \
        --maintenance-policy=TERMINATE \
        --boot-disk-size=100GB \
        --image-family=common-cu121-ubuntu-2204 \
        --image-project=deeplearning-platform-release \
        $SPOT_FLAG \
        --metadata=startup-script='#!/bin/bash
            # Install Docker + NVIDIA Container Toolkit
            curl -fsSL https://get.docker.com | sh
            distribution=$(. /etc/os-release;echo $ID$VERSION_ID)
            curl -fsSL https://nvidia.github.io/libnvidia-container/gpgkey | gpg --dearmor -o /usr/share/keyrings/nvidia-container-toolkit-keyring.gpg
            curl -s -L "https://nvidia.github.io/libnvidia-container/${distribution}/libnvidia-container.list" | \
                sed "s#deb https://#deb [signed-by=/usr/share/keyrings/nvidia-container-toolkit-keyring.gpg] https://#g" | \
                tee /etc/apt/sources.list.d/nvidia-container-toolkit.list
            apt-get update && apt-get install -y nvidia-container-toolkit
            nvidia-ctk runtime configure --runtime=docker
            systemctl restart docker
            echo "READY" > /tmp/cloud-init-done
        '

    echo "Waiting for VM to be ready..."
    sleep 30

    echo ""
    echo "=== VM Created ==="
    echo "SSH:  gcloud compute ssh $VM_NAME --zone=$ZONE"
    echo ""
    echo "Next steps:"
    echo "  1. SSH into the VM"
    echo "  2. Clone your repo or scp your code"
    echo "  3. Run: docker compose -f docker-compose.training.yml up --build"
    echo "  4. Access the API at http://<EXTERNAL_IP>:8000"
    echo ""
    echo "Or use the deploy helper:"
    echo "  gcloud compute scp --recurse . $VM_NAME:~/digimon --zone=$ZONE"
    echo "  gcloud compute ssh $VM_NAME --zone=$ZONE -- 'cd ~/digimon && docker compose -f docker-compose.training.yml up --build -d'"
}

# ── AWS ────────────────────────────────────────────────────────────────

aws_create() {
    local INSTANCE_TYPE="g5.2xlarge"

    if [[ "$NUM_GPUS" -gt 1 ]]; then
        case "$NUM_GPUS" in
            2) INSTANCE_TYPE="g5.12xlarge" ;;  # 4x A10G (smallest multi-GPU)
            4) INSTANCE_TYPE="g5.12xlarge" ;;
            *) INSTANCE_TYPE="g5.48xlarge" ;;   # 8x A10G
        esac
    fi

    local SPOT_FLAG=""
    if $USE_SPOT; then
        SPOT_FLAG="--instance-market-options MarketType=spot"
    fi

    # Use Deep Learning AMI (Ubuntu 22.04) — pre-installed NVIDIA drivers + Docker
    local AMI_ID
    AMI_ID=$(aws ec2 describe-images \
        --owners amazon \
        --filters "Name=name,Values=Deep Learning AMI GPU PyTorch*Ubuntu 22.04*" \
        --query 'sort_by(Images, &CreationDate)[-1].ImageId' \
        --output text)

    echo "Creating AWS instance: $VM_NAME ($INSTANCE_TYPE, spot=$USE_SPOT)"
    echo "AMI: $AMI_ID"

    local INSTANCE_ID
    INSTANCE_ID=$(aws ec2 run-instances \
        --image-id "$AMI_ID" \
        --instance-type "$INSTANCE_TYPE" \
        --key-name "${AWS_KEY_NAME:-digimon-training}" \
        --block-device-mappings "DeviceName=/dev/sda1,Ebs={VolumeSize=100,VolumeType=gp3}" \
        --tag-specifications "ResourceType=instance,Tags=[{Key=Name,Value=$VM_NAME}]" \
        --user-data '#!/bin/bash
            curl -fsSL https://get.docker.com | sh
            apt-get install -y nvidia-container-toolkit
            nvidia-ctk runtime configure --runtime=docker
            systemctl restart docker
            echo "READY" > /tmp/cloud-init-done
        ' \
        $SPOT_FLAG \
        --query 'Instances[0].InstanceId' \
        --output text)

    echo "Instance ID: $INSTANCE_ID"
    echo "Waiting for instance to be running..."
    aws ec2 wait instance-running --instance-ids "$INSTANCE_ID"

    local PUBLIC_IP
    PUBLIC_IP=$(aws ec2 describe-instances \
        --instance-ids "$INSTANCE_ID" \
        --query 'Reservations[0].Instances[0].PublicIpAddress' \
        --output text)

    echo ""
    echo "=== Instance Created ==="
    echo "IP:   $PUBLIC_IP"
    echo "SSH:  ssh -i ~/.ssh/${AWS_KEY_NAME:-digimon-training}.pem ubuntu@$PUBLIC_IP"
    echo ""
    echo "Next steps:"
    echo "  1. Wait ~2 min for cloud-init to finish"
    echo "  2. scp -r . ubuntu@$PUBLIC_IP:~/digimon"
    echo "  3. ssh ubuntu@$PUBLIC_IP 'cd ~/digimon && docker compose -f docker-compose.training.yml up --build -d'"
    echo "  4. Access the API at http://$PUBLIC_IP:8000"
}

# ── SSH ────────────────────────────────────────────────────────────────

do_ssh() {
    if command -v gcloud &>/dev/null; then
        gcloud compute ssh "$VM_NAME" --zone=us-central1-a
    else
        echo "Use: ssh -i ~/.ssh/<key>.pem ubuntu@<IP>"
    fi
}

# ── Teardown ───────────────────────────────────────────────────────────

do_teardown() {
    echo "Tearing down VM: $VM_NAME"
    if command -v gcloud &>/dev/null; then
        gcloud compute instances delete "$VM_NAME" --zone=us-central1-a --quiet || true
    fi
    if command -v aws &>/dev/null; then
        local INSTANCE_ID
        INSTANCE_ID=$(aws ec2 describe-instances \
            --filters "Name=tag:Name,Values=$VM_NAME" "Name=instance-state-name,Values=running,stopped" \
            --query 'Reservations[0].Instances[0].InstanceId' \
            --output text 2>/dev/null) || true
        if [[ -n "$INSTANCE_ID" && "$INSTANCE_ID" != "None" ]]; then
            aws ec2 terminate-instances --instance-ids "$INSTANCE_ID"
            echo "Terminated AWS instance: $INSTANCE_ID"
        fi
    fi
    echo "Done."
}

# ── Dispatch ───────────────────────────────────────────────────────────

case "$PROVIDER" in
    gcp) gcp_create ;;
    aws) aws_create ;;
    ssh) do_ssh ;;
    teardown) do_teardown ;;
    *)
        echo "Usage: $0 {gcp|aws|ssh|teardown} [--gpus N] [--spot] [--concurrent N]"
        echo ""
        echo "Examples:"
        echo "  $0 gcp --spot                    # 1x L4 GPU, spot pricing (~\$0.25/hr)"
        echo "  $0 gcp --gpus 4 --spot            # 4x L4 GPU, spot (~\$1.50/hr)"
        echo "  $0 aws --gpus 4 --spot            # 4x A10G GPU, spot (~\$2.20/hr)"
        echo "  $0 ssh                            # SSH into existing VM"
        echo "  $0 teardown                       # Delete the VM"
        exit 1
        ;;
esac
