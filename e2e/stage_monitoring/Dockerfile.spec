FROM registry.ci.openshift.org/ci/tests-private-base:4.20

WORKDIR /

COPY . .

RUN chmod +x ./check_cincinnati_spec.sh

USER root
CMD source ./check_cincinnati_spec.sh
