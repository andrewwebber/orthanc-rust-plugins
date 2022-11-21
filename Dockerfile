FROM alpine:3 as certs
RUN apk --update add ca-certificates tzdata

FROM scratch
ENV TZ Europe/Berlin
ENV LD_LIBRARY_PATH=/lib64
COPY --from=certs /usr/share/zoneinfo /usr/share/zoneinfo
COPY --from=certs /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=certs /usr/share/zoneinfo/Europe/Berlin /etc/localtime
COPY ./resources/libdcmtk14 /usr/share/libdcmtk14
COPY ./libs/* /lib64/
COPY ./target/release/libs3.so /var/lib/orthanc/plugins/
COPY ./orthanc-gdcm/build/libOrthancGdcm.so /var/lib/orthanc/plugins/
COPY ./orthanc/build/Orthanc /usr/bin/orthanc
COPY ./config/*.json /etc/orthanc/
COPY ./config/docker/*.json /etc/orthanc/
VOLUME /tmp
EXPOSE 8888
ENTRYPOINT ["orthanc"]
CMD ["/etc/orthanc"]
