# Cairn

## Architecture
Using Hexagonal architecture this service will do the following:
    - Query local data using Apache Datafusion
    - Expose a API to perform said queries

### Services
    - DataFusion service (filter push-down)
    - HTTP server