FROM node:21-alpine3.20

WORKDIR /home/app
EXPOSE 3000

COPY dist/bundle.js .

CMD ["node", "bundle.js"]