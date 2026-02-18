FROM node:20-slim
WORKDIR /app
COPY package*.json ./
COPY packages/core/ packages/core/
COPY packages/hosted/ packages/hosted/
RUN npm install --workspace=packages/hosted --omit=dev
EXPOSE 3000
ENV NODE_ENV=production
CMD ["node", "packages/hosted/src/index.js"]
