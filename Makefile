export COMMIT_HASH := $(shell git rev-parse --short=8 HEAD)
export PKG_CONFIG_ALLOW_CROSS=1
export PROJECT_NAME := orthanc-rust-samples
export OCI_REGISTRY := docker.io

all: orthanc build release

.PHONY: libs
libs:
	mkdir -p ./libs
	cp /lib64/ld-linux-x86-64.so.2 ./libs
	cp /usr/lib/libdns_sd.so ./libs
	cp /usr/lib/libresolv.so.2 ./libs
	cp /usr/lib/libnss_files.so.2 ./libs
	cp /usr/lib/libnss_dns.so.2 ./libs
	ldd ./target/release/libs3.so | awk 'BEGIN { FS = "=>" } ; {print $$2;}' | awk 'length {print $$1;}' | while read dep; do cp $$dep ./libs/; done;
	ldd ./orthanc-gdcm/build/libOrthancGdcm.so | awk 'BEGIN { FS = "=>" } ; {print $$2;}' | awk 'length {print $$1;}' | while read dep; do cp $$dep ./libs/; done;
	ldd ./orthanc/build/Orthanc | awk 'BEGIN { FS = "=>" } ; {print $$2;}' | awk 'length {print $$1;}' | while read dep; do cp $$dep ./libs/; done;

orthanc:
	./hack/orthanc

audit:
	mkdir -p ./target/audit || true
	cargo audit --ignore RUSTSEC-2020-0159 --ignore RUSTSEC-2020-0071

build: audit orthanc-plugin-bindings/src/bindings.rs
	cargo clippy --tests -- -Dwarnings

test:
	cargo test -- --nocapture

release:
	cargo build --release

run-orthanc:
	./orthanc/build/Orthanc --trace --trace-dicom ./config

run-orthanc-docker:
	docker run --rm -it --net=host -v $$PWD/.env:/.env:ro orthanc-rust-samples:${COMMIT_HASH}

docker-image: libs
	docker build --network=host --no-cache -t ${OCI_REGISTRY}/${PROJECT_NAME}:${COMMIT_HASH} .
	trivy image --exit-code 1 --severity HIGH,CRITICAL ${OCI_REGISTRY}/${PROJECT_NAME}:${COMMIT_HASH}

docker-push: docker-image
	docker push ${OCI_REGISTRY}/${PROJECT_NAME}:${COMMIT_HASH}

orthanc-plugin-bindings/src/bindings.rs:
	bindgen ./orthanc/OrthancServer/Plugins/Include/orthanc/OrthancCPlugin.h -o ./orthanc-plugin-bindings/src/bindings.rs --allowlist-function="Orthanc.*" --allowlist-type="Orthanc.*" --allowlist-var="Orthanc.*" --no-layout-tests
