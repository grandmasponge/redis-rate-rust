# Redis Rate limiter in rust (

this is a project for a freind for is api endpoints that need rate limiting, 
hopefully i can make this so that all users can successfully impliment rate limiting middleware into their axum or any 
rust web framework.

### how it works

essentially rate limiting can be done in many ways but the way i have implimented this for my use case is
like a bucket, a bucket for each ip address or token, once that bucket overflows you are given the http response for to many requests
and you will have to wait until essentially that bucket drains.



